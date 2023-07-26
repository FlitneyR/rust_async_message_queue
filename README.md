# rust_async_message_queue

is a library for asynchronous communication between threads via queues.

Each asynchronous queue containings a queue, a queue state, and a list
of registered writers. Writing to the queue requires a writter ID. When
every writer has de-registered, the queue closes, and then when every
remaining message has been read, the queue terminates.

When a closed queue is read from, it will return an `EndOfTransmission` error,
every subsequent read will return a `QueueTerminated` error.
