use std::{sync::{Arc, Mutex}, ops::{AddAssign, SubAssign}};

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    pub fn one_writer_one_reader() {
        let queue = AsyncMsgQueue::<String>::new_arc();

        let reader = queue.clone();
        let writer = queue.clone();

        let thread_handle = std::thread::spawn(move || {
            let mut messages = vec![];

            while reader.can_read()? {
                reader.read()
                    .map(|msg| messages.push(msg));
            }

            Some(messages)
        });

        let messages = vec!["msg1".into(), "msg2".into(), "msg3".into()];

        writer.register_writer();

        for message in messages.clone() {
            assert_eq!(writer.send(message), Some(()));
        }

        writer.deregister_writer();

        let result = thread_handle.join();

        assert!(result.is_ok());

        let result = result.unwrap();

        assert_eq!(result, Some(messages))
    }

    #[test]
    pub fn one_writer_two_readers() {
        let queue = AsyncMsgQueue::<usize>::new_arc();

        let writer = queue.clone();
        let reader1 = queue.clone();
        let reader2 = queue.clone();

        let handle1 = std::thread::spawn(move || {
            let mut acc = 0;
            while reader1.can_read()? {
                reader1.read()
                    .map(|msg| acc += msg);
            }
            Some(acc)
        });

        let handle2 = std::thread::spawn(move || {
            let mut acc = 0;
            while reader2.can_read()? {
                reader2.read()
                    .map(|msg| acc += msg);
            }
            Some(acc)
        });

        let arr: Vec<usize> = (0..100).collect();

        writer.register_writer();

        for n in arr.clone() {
            assert_eq!(writer.send(n), Some(()));
        }

        writer.deregister_writer();

        let result1 = handle1.join();
        let result2 = handle2.join();

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let result1 = result1.unwrap();
        let result2 = result2.unwrap();

        assert!(result1.is_some());
        assert!(result2.is_some());

        let result1 = result1.unwrap();
        let result2 = result2.unwrap();

        assert_eq!(result1 + result2, arr.iter().sum())
    }

    #[test]
    pub fn two_writers_one_reader() {
        let queue = AsyncMsgQueue::<usize>::new_arc();
        let writer1 = queue.clone();
        let writer2 = queue.clone();
        let reader = queue.clone();

        let reader_handle = std::thread::spawn(move || {
            let mut acc = 0;

            while reader.can_read()? {
                reader.read()
                    .map(|v| acc += v);
            }

            Some(acc)
        });

        let arr1 = vec![1, 2, 3];
        let arr2 = vec![4, 5, 6];

        writer1.register_writer();
        writer2.register_writer();

        std::thread::spawn(move || {
            for n in arr1 {
                writer1.send(n)?;
            }

            writer1.deregister_writer();

            Some(())
        });

        std::thread::spawn(move || {
            for n in arr2 {
                writer2.send(n)?;
            }

            writer2.deregister_writer();

            Some(())
        });

        let result = reader_handle.join();

        assert!(result.is_ok());

        let result = result.unwrap();

        assert!(result.is_some());

        let result = result.unwrap();

        assert_eq!(result, vec![1, 2, 3, 4, 5, 6].iter().sum());
    }

    #[test]
    pub fn two_writers_two_reader() {
        let queue = AsyncMsgQueue::<usize>::new_arc();
        let writer1 = queue.clone();
        let writer2 = queue.clone();
        let reader1 = queue.clone();
        let reader2 = queue.clone();

        writer1.register_writer();
        writer2.register_writer();

        let reader_handle1 = std::thread::spawn(move || {
            let mut acc = 0;

            while reader1.can_read()? {
                reader1.read()
                    .map(|v| acc += v);
            }

            Some(acc)
        });

        let reader_handle2 = std::thread::spawn(move || {
            let mut acc = 0;

            while reader2.can_read()? {
                reader2.read()
                    .map(|v| acc += v);
            }

            Some(acc)
        });

        std::thread::spawn(move || {
            for n in vec![1, 2, 3] {
                writer1.send(n)?;
            }

            writer1.deregister_writer();

            Some(())
        });

        std::thread::spawn(move || {
            for n in vec![4, 5, 6] {
                writer2.send(n)?;
            }

            writer2.deregister_writer();

            Some(())
        });

        let result1 = reader_handle1.join();
        let result2 = reader_handle2.join();

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let result1 = result1.unwrap();
        let result2 = result2.unwrap();

        assert!(result1.is_some());
        assert!(result2.is_some());

        let result1 = result1.unwrap();
        let result2 = result2.unwrap();

        assert_eq!(result1 + result2, 1 + 2 + 3 + 4 + 5 + 6);
    }
}

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
