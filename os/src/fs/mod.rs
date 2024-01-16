pub mod inode;
pub mod null;
pub mod pipe;
pub mod stdio;
pub mod zero;
use crate::{
    mm::address::UserBuffer,
    syscall::{UNREADABLE, UNSEEKABLE, UNWRITABLE},
};

pub trait File: Send + Sync {
    fn readable(&self) -> bool {
        false
    }
    fn writable(&self) -> bool {
        false
    }
    fn seekable(&self) -> bool {
        false
    }
    fn read(&self, _: UserBuffer) -> isize {
        UNREADABLE
    }
    fn write(&self, _: UserBuffer) -> isize {
        UNWRITABLE
    }
    fn seek(&self, _: SeekType, _: i32) -> isize {
        UNSEEKABLE
    }
}

#[derive(Clone, Copy)]
pub enum SeekType {
    Set = 0,
    Cur = 1,
    End = 2,
}
