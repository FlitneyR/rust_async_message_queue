use std::{
    sync::{Arc, Mutex},
    ops::{AddAssign, SubAssign}
};

#[cfg(test)]
mod tests;

pub struct Queue<T> {
    vec: Vec<T>
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Self { vec: Vec::new() }
    }

    pub fn push(&mut self, t: T) {
        self.vec.insert(0, t)
    }

    pub fn pop(&mut self) -> Option<T> {
        self.vec.pop()
    }

    pub fn peek(&mut self) -> Option<&T> {
        self.vec.get(self.vec.len())
    }
}

/// The message queue can be in one of three states:
/// - Open - The message queue can recieve more messages
/// - Closed - The message queue can recieve no more messages
/// - Terminated - The message queue has been closed and all the messages have been read
#[derive(PartialEq)]
pub enum MsgQueueState {
    Open,
    Closed,
    Terminated,
}

impl MsgQueueState {
    pub fn new() -> Self { Self::Open }

    pub fn close(&mut self) {
        *self = Self::Closed
    }

    pub fn terminate(&mut self) {
        *self = Self::Terminated
    }

    fn can_send(&self) -> bool {
        *self == Self::Open
    }

    fn can_read(&self) -> bool {
        *self != Self::Terminated
    }
}

#[derive(PartialEq, Debug)]
pub enum MsgQueueError {
    NoLock,
    NoMessages,
    QueueClosed,
    NegativeWriters,
    QueueTerminated,
    EndOfTransmission,
} use MsgQueueError::*;

pub struct AsyncMsgQueue<T> {
    queue: Mutex<Queue<Option<T>>>,
    state: Mutex<MsgQueueState>,
    writers: Mutex<usize>,
}

impl<T> AsyncMsgQueue<T> {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(Queue::new()),
            state: Mutex::new(MsgQueueState::new()),
            writers: Mutex::new(0)
        }
    }

    pub fn register_writer(&self) -> Result<(), MsgQueueError> {
        self.writers
            .lock().map_err(|_| NoLock)?
            .add_assign(1);

        Ok(())
    }

    pub fn deregister_writer(&self) -> Result<(), MsgQueueError> {
        let mut lock = self.writers
            .lock().map_err(|_| NoLock)?;

        lock.sub_assign(1);

        if lock.eq(&0) {
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

    /// Prevent any readers from reading any more messages
    pub fn terminate(&self) -> Result<(), MsgQueueError> {
        Ok(self.state.lock().map_err(|_| NoLock)?.terminate())
    }

    /// Prevent any writers from sending any more messages
    fn close(&self) -> Result<(), MsgQueueError> {
        if self.is_closed()? { return Err(QueueClosed) }

        self.queue
            .lock().map_err(|_| NoLock)?
            .push(None);

        self.state
            .lock().map_err(|_| NoLock)?
            .close();

        Ok(())
    }

    /// Enqueues a message
    pub fn send(&self, t: T) -> Result<(), MsgQueueError> {
        if !self.can_send()? { return Err(QueueClosed) }

        self.queue
            .lock().map_err(|_| NoLock)?
            .push(Some(t));
        Ok(())
    }

    fn pop(&self) -> Result<T, MsgQueueError> {
        if self.is_terminated()? { return Err(QueueTerminated) }

        let temp = self.queue
            .lock().map_err(|_| NoLock)?
            .pop();

        match temp {
            None => Err(NoMessages),
            Some(None) => { self.terminate()?; Err(EndOfTransmission) },
            Some(Some(v)) => Ok(v)
        }
    }

    /// Reads the next message from the queue
    /// 
    /// If there are no messages, this function will busy wait for one
    pub fn read(&self) -> Result<T, MsgQueueError> {
        let mut temp;
        while {
            temp = self.pop();

            match temp {
                Ok(_) => false,
                Err(EndOfTransmission) => false,
                Err(QueueTerminated) => false,
                _ => true
            }
        } { /* busy wait */ }

        temp
    }
}
