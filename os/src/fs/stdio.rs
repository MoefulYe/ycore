use alloc::sync::Arc;

use crate::{process::processor::PROCESSOR, sbi::console_getchar};

use super::File;

pub const STDIN: usize = 0;
pub const STDOUT: usize = 1;
pub const STDERR: usize = 2;

struct Stdin;
struct Stdout;
struct Stderr;

impl File for Stdin {
    fn readable(&self) -> bool {
        true
    }

    fn read(&self, mut user_buf: crate::mm::address::UserBuffer) -> isize {
        assert!(user_buf.len() == 1);
        let mut c: usize;
        loop {
            c = console_getchar();
            if c == 0 {
                PROCESSOR.exclusive_access().suspend_current().schedule();
                continue;
            } else {
                break;
            }
        }
        *user_buf.next().unwrap().first_mut().unwrap() = c as u8;
        1
    }
}

impl File for Stdout {
    fn writable(&self) -> bool {
        true
    }

    fn write(&self, buf: crate::mm::address::UserBuffer) -> isize {
        let len = buf.len();
        for buf in buf {
            let s = core::str::from_utf8(buf).unwrap();
            print!("{}", s);
        }
        len as isize
    }
}

impl File for Stderr {
    fn writable(&self) -> bool {
        true
    }

    fn write(&self, buf: crate::mm::address::UserBuffer) -> isize {
        let len = buf.len();
        for buf in buf {
            let s = core::str::from_utf8(buf).unwrap();
            print!("{}", s);
        }
        len as isize
    }
}

pub fn stdin() -> Arc<dyn File + Send + Sync> {
    Arc::new(Stdin)
}

pub fn stdout() -> Arc<dyn File + Send + Sync> {
    Arc::new(Stdout)
}

pub fn stderr() -> Arc<dyn File + Send + Sync> {
    Arc::new(Stderr)
}
