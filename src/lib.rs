use std::{sync::{Arc, Mutex}, ops::{AddAssign, SubAssign}};

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

    pub fn register_writer(&self) -> Option<()> {
        self.writers
            .lock().ok()?
            .add_assign(1);

        Some(())
    }

    pub fn deregister_writer(&self) -> Option<()> {
        let mut lock = self.writers.lock().ok()?;

        lock.sub_assign(1);

        if lock.eq(&0) {
            self.close()?
        }

        Some(())
    }

    pub fn new_arc() -> Arc<Self> { Arc::new(Self::new()) }

    pub fn is_closed(&self) -> Option<bool> {
        self.can_send().map(|v| !v)
    }

    pub fn is_terminated(&self) -> Option<bool> {
        self.can_read().map(|v| !v)
    }

    pub fn can_send(&self) -> Option<bool> {
        Some(self.state.lock().ok()?.can_send())
    }

    pub fn can_read(&self) -> Option<bool> {
        Some(self.state.lock().ok()?.can_read())
    }

    /// Prevent any readers from reading any more messages
    pub fn terminate(&self) -> Option<()> {
        Some(self.state.lock().ok()?.terminate())
    }

    /// Prevent any writers from sending any more messages
    fn close(&self) -> Option<()> {
        if self.is_closed()? { return Some(()) }

        self.queue
            .lock().ok()?
            .push(None);

        self.state
            .lock().ok()?
            .close();

        Some(())
    }

    /// Enqueues a message
    pub fn send(&self, t: T) -> Option<()> {
        if !self.can_send()? { return None }

        self.queue
            .lock().ok()?
            .push(Some(t));
        Some(())
    }

    fn pop(&self) -> Option<Option<T>> {
        if self.is_terminated()? { return None }

        let temp = self.queue
            .lock().ok()?
            .pop();

        match temp {
            Some(None) => { self.terminate()?; None },
            _ => temp
        }
    }

    /// Reads the next message from the queue
    /// 
    /// If there are no messages, this function will busy wait for one
    pub fn read(&self) -> Option<T> {
        let mut temp;
        while {
            temp = self.pop();
            self.can_read()? && temp.is_none()
        } { /* busy wait */ }

        match temp {
            None => None, // queue was closed
            Some(v) => v  // message was recieved
        }
    }
}

impl<T> Iterator for AsyncMsgQueue<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.read()
    }
}
