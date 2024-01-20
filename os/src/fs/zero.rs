use alloc::sync::Arc;

use super::File;

struct Zero;

impl File for Zero {
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
        let len = buf.len();
        for buf in buf {
            buf.fill(0);
        }
        len as isize
    }

    fn write(&self, _: crate::mm::address::UserBuffer) -> isize {
        0
    }

    fn seek(&self, _: super::SeekType, _: i32) -> isize {
        0
    }
}

pub fn zero() -> Arc<dyn File + Send + Sync> {
    Arc::new(Zero)
}
