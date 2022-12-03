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