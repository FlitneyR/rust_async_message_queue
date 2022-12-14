use rand::Rng;
use std::sync::{ Arc, Mutex };

#[cfg(test)]
mod tests;

struct Queue<T> {
    vec: Vec<T>
}

impl<T> Queue<T> {
    fn new() -> Self {
        Self { vec: Vec::new() }
    }

    fn push(&mut self, t: T) {
        self.vec.insert(0, t)
    }

    fn pop(&mut self) -> Option<T> {
        self.vec.pop()
    }
}

/// The message queue can be in one of three states:
/// - Open - The message queue can recieve more messages
/// - Closed - The message queue can recieve no more messages
/// - Terminated - The message queue has been closed and all the messages have been read
#[derive(PartialEq)]
enum MsgQueueState {
    Open,
    Closed,
    Terminated,
}

impl MsgQueueState {
    pub fn new() -> Self { Self::Open }

    fn close(&mut self) {
        *self = Self::Closed
    }

    fn terminate(&mut self) {
        *self = Self::Terminated
    }

    fn can_send(&self) -> bool {
        *self == Self::Open
    }

    fn can_read(&self) -> bool {
        *self != Self::Terminated
    }
}

// TODO: add more information to MsgQueueError
#[derive(PartialEq, Debug)]
pub enum MsgQueueError {
    NoLock,
    NoMessages,
    QueueClosed,
    UnknownWriter,
    NegativeWriters,
    QueueTerminated,
    EndOfTransmission,
} use MsgQueueError::*;

impl MsgQueueError {
    pub fn to_string(&self) -> String {
        match self {
            NoLock => "Failed to get mutex lock".into(),
            NoMessages => "No messages to read".into(),
            QueueClosed => "Cannot send to closed queue".into(),
            UnknownWriter => "Unrecognised writer".into(),
            NegativeWriters => "Cannot have fewer than 1 writers to a queue".into(),
            QueueTerminated => "Cannot read from terminated queue".into(),
            EndOfTransmission => "Message queue reached end of transmission".into(),
        }
    }
}

type WriterID = usize;

// TODO: Add names to message queues
pub struct AsyncMsgQueue<T> {
    queue: Mutex<Queue<T>>,
    state: Mutex<MsgQueueState>,
    writers: Mutex<Vec<WriterID>>,
}

/// ```
/// use async_msg_queue::{
///     AsyncMsgQueue,
///     MsgQueueError::*
/// };
/// 
/// let queue = AsyncMsgQueue::<String>::new_arc();
/// 
/// let reader = queue.clone();
/// let writer = queue.clone();
/// 
/// let thread_handle = std::thread::spawn(move || {
///     let mut messages = vec![];
/// 
///     loop {
///         match reader.read() {
///             Ok(msg) => messages.push(msg),
///             Err(EndOfTransmission) |
///             Err(QueueTerminated) => return Ok(messages),
///             Err(e) => return Err(e)
///         }
///     }
/// });
/// 
/// let messages = vec!["msg1".into(), "msg2".into(), "msg3".into()];
/// 
/// let writer_handle = writer.register_writer();
/// 
/// assert!(writer_handle.is_ok());
/// 
/// let writer_handle = writer_handle.unwrap();
/// 
/// for message in messages.clone() {
///     assert_eq!(writer.send(writer_handle, message), Ok(()));
/// }
/// 
/// assert_eq!(writer.deregister_writer(writer_handle), Ok(()));
/// 
/// let result = thread_handle.join();
/// 
/// assert!(result.is_ok());
/// 
/// let result = result.unwrap();
/// 
/// assert_eq!(result, Ok(messages))
/// ```
impl<T> AsyncMsgQueue<T> {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(Queue::new()),
            state: Mutex::new(MsgQueueState::new()),
            writers: Mutex::new(Vec::new())
        }
    }

    fn new_writer_id(&self) -> WriterID {
        rand::thread_rng().gen()
    }

    fn check_writer(&self, id: WriterID) -> Result<(), MsgQueueError> {
        if self.writers
            .lock().map_err(|_| NoLock)?
            .contains(&id)
        {
            Ok(())
        } else {
            Err(UnknownWriter)
        }
    }

    pub fn register_writer(&self) -> Result<WriterID, MsgQueueError> {
        let id = self.new_writer_id();

        self.writers
            .lock().map_err(|_| NoLock)?
            .push(id);

        Ok(id)
    }

    pub fn deregister_writer(&self, id: WriterID) -> Result<(), MsgQueueError> {
        let mut writers = self.writers
            .lock().map_err(|_| NoLock)?;

        let index = writers.iter()
            .position(|&writer| writer == id)
            .ok_or(UnknownWriter)?;

        writers.remove(index);

        if writers.is_empty() {
            self.close()?
        }

        Ok(())
    }

    pub fn new_arc() -> Arc<Self> { Arc::new(Self::new()) }

    pub fn is_closed(&self) -> Result<bool, MsgQueueError> {
        self.can_send().map(|v| !v)
    }

    pub fn is_terminated(&self) -> Result<bool, MsgQueueError> {
        self.can_read().map(|v| !v)
    }

    pub fn can_send(&self) -> Result<bool, MsgQueueError> {
        Ok(self.state.lock().map_err(|_| NoLock)?.can_send())
    }

    pub fn can_read(&self) -> Result<bool, MsgQueueError> {
        Ok(self.state.lock().map_err(|_| NoLock)?.can_read())
    }

    fn terminate(&self) -> Result<(), MsgQueueError> {
        Ok(self.state.lock().map_err(|_| NoLock)?.terminate())
    }

    /// Prevent any writers from sending any more messages
    fn close(&self) -> Result<(), MsgQueueError> {
        if self.is_closed()? { return Err(QueueClosed) }

        self.state
            .lock().map_err(|_| NoLock)?
            .close();

        Ok(())
    }

    /// Enqueues a message
    pub fn send(&self, id: WriterID, t: T) -> Result<(), MsgQueueError> {
        self.check_writer(id)?;

        if !self.can_send()? { return Err(QueueClosed) }

        self.queue
            .lock().map_err(|_| NoLock)?
            .push(t);

        Ok(())
    }

    fn pop(&self) -> Result<T, MsgQueueError> {
        if self.is_terminated()? { return Err(QueueTerminated) }

        let mut lock = self.queue
            .lock().map_err(|_| NoLock)?;

        match lock.pop() {
            Some(v) => Ok(v),
            None => if self.is_closed()? {
                self.terminate()?;
                Err(EndOfTransmission)
            } else {
                Err(NoMessages)
            },
        }
    }

    /// Reads the next message from the queue
    /// 
    /// If there are no messages, this function will busy wait for one
    pub fn read(&self) -> Result<T, MsgQueueError> {
        loop {
            match self.pop() {
                Err(NoMessages) => continue,
                Ok(v) => return Ok(v),
                Err(e) => return Err(e),
            }
        }
    }
}
