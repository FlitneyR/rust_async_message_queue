use crate::*;

#[test]
pub fn one_writer_one_reader() {
    let queue = AsyncMsgQueue::<String>::new_arc();

    let reader = queue.clone();
    let writer = queue.clone();

    let thread_handle = std::thread::spawn(move || {
        let mut messages = vec![];

        loop {
            match reader.read() {
                Ok(msg) => messages.push(msg),
                Err(EndOfTransmission) |
                Err(QueueTerminated) => return Ok(messages),
                Err(e) => return Err(e)
            }
        }
    });

    let messages = vec!["msg1".into(), "msg2".into(), "msg3".into()];

    let writer_handle = writer.register_writer();

    assert!(writer_handle.is_ok());

    let writer_handle = writer_handle.unwrap();

    for message in messages.clone() {
        assert_eq!(writer.send(writer_handle, message), Ok(()));
    }

    assert_eq!(writer.deregister_writer(writer_handle), Ok(()));

    let result = thread_handle.join();

    assert!(result.is_ok());

    let result = result.unwrap();

    assert_eq!(result, Ok(messages))
}

#[test]
pub fn one_writer_two_readers() {
    let queue = AsyncMsgQueue::<usize>::new_arc();

    let writer = queue.clone();
    let reader1 = queue.clone();
    let reader2 = queue.clone();

    let handle1 = std::thread::spawn(move || {
        let mut acc = 0;
        loop {
            match reader1.read() {
                Ok(msg) => acc += msg,
                Err(EndOfTransmission) |
                Err(QueueTerminated) => return Ok(acc),
                Err(e) => return Err(e)
            }
        }
    });

    let handle2 = std::thread::spawn(move || {
        let mut acc = 0;
        loop {
            match reader2.read() {
                Ok(msg) => acc += msg,
                Err(EndOfTransmission) |
                Err(QueueTerminated) => return Ok(acc),
                Err(e) => return Err(e)
            }
        }
    });

    let arr: Vec<usize> = (0..100).collect();

    let writer_handle = writer.register_writer();

    assert!(writer_handle.is_ok());

    let writer_handle = writer_handle.unwrap();

    for n in arr.clone() {
        assert_eq!(writer.send(writer_handle, n), Ok(()));
    }

    assert_eq!(writer.deregister_writer(writer_handle), Ok(()));

    let result1 = handle1.join();
    let result2 = handle2.join();

    assert!(result1.is_ok());
    assert!(result2.is_ok());

    let result1 = result1.unwrap();
    let result2 = result2.unwrap();

    assert!(result1.is_ok());
    assert!(result2.is_ok());

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

        loop {
            match reader.read() {
                Ok(v) => acc += v,
                Err(EndOfTransmission) |
                Err(QueueTerminated) => return Ok(acc),
                Err(e) => return Err(e)
            }
        }
    });

    let arr1 = vec![1, 2, 3];
    let arr2 = vec![4, 5, 6];

    let writer1_handle = writer1.register_writer();
    let writer2_handle = writer2.register_writer();

    assert!(writer1_handle.is_ok());
    assert!(writer2_handle.is_ok());

    let writer1_handle = writer1_handle.unwrap();
    let writer2_handle = writer2_handle.unwrap();

    std::thread::spawn(move || {
        for n in arr1 {
            writer1.send(writer1_handle, n)?;
        }

        writer1.deregister_writer(writer1_handle)?;

        Ok::<(), MsgQueueError>(())
    });

    std::thread::spawn(move || {
        for n in arr2 {
            writer2.send(writer2_handle, n)?;
        }

        writer2.deregister_writer(writer2_handle)?;

        Ok::<(), MsgQueueError>(())
    });

    let result = reader_handle.join();

    assert!(result.is_ok());

    let result = result.unwrap();

    assert!(result.is_ok());

    let result = result.unwrap();

    assert_eq!(result, vec![1, 2, 3, 4, 5, 6].iter().sum());
}

#[test]
pub fn two_writers_two_readers() {
    let queue = AsyncMsgQueue::<usize>::new_arc();
    let writer1 = queue.clone();
    let writer2 = queue.clone();
    let reader1 = queue.clone();
    let reader2 = queue.clone();

    let writer1_handle = writer1.register_writer();
    let writer2_handle = writer2.register_writer();

    assert!(writer1_handle.is_ok());
    assert!(writer2_handle.is_ok());

    let writer1_handle = writer1_handle.unwrap();
    let writer2_handle = writer2_handle.unwrap();

    let reader_handle1 = std::thread::spawn(move || {
        let mut acc = 0;

        loop {
            match reader1.read() {
                Ok(v) => acc += v,
                Err(EndOfTransmission) |
                Err(QueueTerminated) => return Ok(acc),
                Err(e) => return Err(e)
            }
        }
    });

    let reader_handle2 = std::thread::spawn(move || {
        let mut acc = 0;

        loop {
            match reader2.read() {
                Ok(v) => acc += v,
                Err(EndOfTransmission) |
                Err(QueueTerminated) => return Ok(acc),
                Err(e) => return Err(e)
            }
        }
    });

    std::thread::spawn(move || {
        for n in vec![1, 2, 3] {
            writer1.send(writer1_handle, n)?;
        }

        writer1.deregister_writer(writer1_handle)?;

        Ok::<(), MsgQueueError>(())
    });

    std::thread::spawn(move || {
        for n in vec![4, 5, 6] {
            writer2.send(writer2_handle, n)?;
        }

        writer2.deregister_writer(writer2_handle)?;

        Ok::<(), MsgQueueError>(())
    });

    let result1 = reader_handle1.join();
    let result2 = reader_handle2.join();

    assert!(result1.is_ok());
    assert!(result2.is_ok());

    let result1 = result1.unwrap();
    let result2 = result2.unwrap();

    assert!(result1.is_ok());
    assert!(result2.is_ok());

    let result1 = result1.unwrap();
    let result2 = result2.unwrap();

    assert_eq!(result1 + result2, 1 + 2 + 3 + 4 + 5 + 6);
}