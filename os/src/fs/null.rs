use alloc::sync::Arc;

use super::File;

struct Null;

impl File for Null {
    fn readable(&self) -> bool {
        true
    }

    fn writable(&self) -> bool {
        true
    }

    fn seekable(&self) -> bool {
        true
    }

    fn read(&self, buf: crate::mm::address::UserBuffer) -> isize {
        buf.len() as isize
    }

    fn write(&self, buf: crate::mm::address::UserBuffer) -> isize {
        buf.len() as isize
    }

    fn seek(&self, _: super::SeekType, _: i32) -> isize {
        0
    }
}

pub fn null() -> Arc<dyn File + Send + Sync> {
    Arc::new(Null)
}
