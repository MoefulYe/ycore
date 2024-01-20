use alloc::sync::{Arc, Weak};

use crate::{
    mm::address::UserBuffer, process::processor::PROCESSOR, sync::up::UPSafeCell,
    syscall::PIPE_READER_CLOSED,
};

use super::File;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PipeState {
    Full,
    Empty,
    Normal,
}

use PipeState::*;

const PIPE_SIZE: usize = 32;

struct Pipe {
    buffer: [u8; PIPE_SIZE],
    head: usize,
    tail: usize,
    state: PipeState,
    write_end: Option<Weak<PipeWriter>>,
    read_end: Option<Weak<PipeReader>>,
}

impl Pipe {
    fn new() -> Arc<UPSafeCell<Self>> {
        unsafe {
            Arc::new(UPSafeCell::new(Pipe {
                buffer: [0; PIPE_SIZE],
                head: 0,
                tail: 0,
                state: Empty,
                write_end: None,
                read_end: None,
            }))
        }
    }

    fn set_write_end(&mut self, write_end: &Arc<PipeWriter>) -> &mut Self {
        self.write_end = Some(Arc::downgrade(write_end));
        self
    }

    fn set_read_end(&mut self, read_end: &Arc<PipeReader>) -> &mut Self {
        self.read_end = Some(Arc::downgrade(read_end));
        self
    }

    fn read_byte(&mut self) -> u8 {
        self.state = Normal;
        let c = self.buffer[self.head];
        self.head = (self.head + 1) % PIPE_SIZE;
        if self.head == self.tail {
            self.state = Empty;
        }
        c
    }

    fn write_byte(&mut self, byte: u8) {
        self.state = Normal;
        self.buffer[self.tail] = byte;
        self.tail = (self.tail + 1) % PIPE_SIZE;
        if self.tail == self.head {
            self.state = Full;
        }
    }

    fn available_to_read(&self) -> usize {
        if self.state == Empty {
            0
        } else if self.tail > self.head {
            self.tail - self.head
        } else {
            self.tail + PIPE_SIZE - self.head
        }
    }

    fn available_to_write(&self) -> usize {
        if self.state == Full {
            0
        } else if self.tail < self.head {
            self.head - self.tail
        } else {
            self.head + PIPE_SIZE - self.tail
        }
    }

    /// 判断写入端是否关闭
    fn is_writer_closed(&self) -> bool {
        self.write_end.as_ref().unwrap().upgrade().is_none()
    }

    fn is_reader_closed(&self) -> bool {
        self.read_end.as_ref().unwrap().upgrade().is_none()
    }
}

struct PipeReader(Arc<UPSafeCell<Pipe>>);

impl PipeReader {
    fn new(pipe: Arc<UPSafeCell<Pipe>>) -> Arc<Self> {
        Arc::new(Self(pipe))
    }
}

struct PipeWriter(Arc<UPSafeCell<Pipe>>);

impl PipeWriter {
    fn new(pipe: Arc<UPSafeCell<Pipe>>) -> Arc<Self> {
        Arc::new(Self(pipe))
    }
}

impl File for PipeReader {
    fn readable(&self) -> bool {
        true
    }

    fn read(&self, user_buf: UserBuffer) -> isize {
        let to_read = user_buf.len();
        let mut read = 0usize;
        let mut it = user_buf.flat_map(|buf| buf.iter_mut());
        loop {
            let pipe = self.0.exclusive_access();
            let this_read = pipe.available_to_read();
            if this_read == 0 {
                if pipe.is_writer_closed() {
                    return read as isize;
                }
                drop(pipe);
                PROCESSOR.exclusive_access().suspend_current().schedule();
                continue;
            }
            for _ in 0..this_read {
                if let Some(byte) = it.next() {
                    *byte = pipe.read_byte();
                    read += 1;
                    if read == to_read {
                        return read as isize;
                    }
                } else {
                    return read as isize;
                }
            }
        }
    }
}

impl File for PipeWriter {
    fn writable(&self) -> bool {
        true
    }
    fn write(&self, user_buf: UserBuffer) -> isize {
        let to_write = user_buf.len();
        let mut write = 0usize;
        let mut it = user_buf.flat_map(|buf| buf.iter_mut());
        loop {
            let pipe = self.0.exclusive_access();
            if pipe.is_reader_closed() {
                return PIPE_READER_CLOSED;
            }
            let this_write = pipe.available_to_write();
            if this_write == 0 {
                drop(pipe);
                PROCESSOR.exclusive_access().suspend_current().schedule();
                continue;
            }
            for _ in 0..this_write {
                if let Some(&mut byte) = it.next() {
                    pipe.write_byte(byte);
                    write += 1;
                    if write == to_write {
                        return write as isize;
                    }
                } else {
                    return write as isize;
                }
            }
        }
    }
}

/// (读取端, 写入端)
pub fn make_pipe() -> (Arc<dyn File + Send + Sync>, Arc<dyn File + Send + Sync>) {
    let pipe = Pipe::new();
    let reader = PipeReader::new(pipe.clone());
    let writer = PipeWriter::new(pipe.clone());
    pipe.exclusive_access()
        .set_read_end(&reader)
        .set_write_end(&writer);
    (reader, writer)
}
