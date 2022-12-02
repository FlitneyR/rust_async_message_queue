use std::sync::Mutex;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
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

pub struct AsyncMsgQueue<T> {
    queue: Mutex<Queue<Option<T>>>
}

impl<T> AsyncMsgQueue<T> {
    pub fn new() -> Self {
        Self { queue: Mutex::new(Queue::new()) }
    }

    pub fn close(&self) -> Option<()> {
        self.queue
            .lock().ok()?
            .push(None);
        Some(())
    }

    pub fn push(&self, t: T) -> Option<()> {
        self.queue
            .lock().ok()?
            .push(Some(t));
        Some(())
    }

    pub fn pop(&self) -> Option<Option<T>> {
        self.queue
            .lock().ok()?
            .pop()
    }

    pub fn await_message(&self) -> Option<T> {
        let mut temp;
        while {
            temp = self.pop();
            temp.is_none()
        } { /* busy wait */ }
        temp.unwrap()
    }
}
