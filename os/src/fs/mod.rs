pub mod inode;
use crate::mm::address::UserBuffer;

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
    fn read(&self, buf: UserBuffer) -> isize {
        io_error::UNREADABLE
    }
    fn write(&self, buf: UserBuffer) -> isize {
        io_error::UNWRITABLE
    }
    fn seek(&self, ty: SeekType, offset: i32) -> isize {
        io_error::UNSEEKABLE
    }
}

#[derive(Clone, Copy)]
pub enum SeekType {
    Set = 0,
    Cur = 1,
    End = 2,
}

pub mod io_error {
    pub const EOF: isize = 0;
    pub const UNREADABLE: isize = -1;
    pub const UNWRITABLE: isize = -2;
    pub const UNSEEKABLE: isize = -3;
    pub const SEEK_OUT_OF_RANGE: isize = -4;
}
