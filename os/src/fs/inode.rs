use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::bitflags;
use yfs::vfs::Vnode;
use yfs::yfs::YeFs;

use crate::drivers::block::BLOCK_DEVICE;
use crate::fs::SeekType;
use crate::sync::up::UPSafeCell;

use crate::mm::address::UserBuffer;

use super::io_error::SEEK_OUT_OF_RANGE;
use super::File;

pub struct OSInode {
    flags: OSInodeFlags,
    inner: UPSafeCell<OSInodeInner>,
}

impl File for OSInode {
    fn read(&self, buf: UserBuffer) -> isize {
        let mut inner = self.inner.exclusive_access();
        let mut total = 0u32;
        for buf in buf {
            let read = inner.inode.read(inner.offset, buf);
            if read == 0 {
                break;
            }
            inner.offset += read;
            total += read;
        }
        total as isize
    }

    fn write(&self, buf: UserBuffer) -> isize {
        let mut inner = self.inner.exclusive_access();
        let mut total = 0u32;
        for buf in buf {
            let write = inner.inode.write(inner.offset, buf);
            if write == 0 {
                break;
            }
            inner.offset += write;
            total += write;
        }
        total as isize
    }

    fn readable(&self) -> bool {
        self.flags.contains(OSInodeFlags::READABLE)
    }

    fn writable(&self) -> bool {
        self.flags.contains(OSInodeFlags::WRITABLE)
    }

    fn seekable(&self) -> bool {
        true
    }

    fn seek(&self, ty: super::SeekType, offset: i32) -> isize {
        let mut inner = self.inner.exclusive_access();
        let to = match ty {
            super::SeekType::Set => offset,
            super::SeekType::Cur => inner.offset as i32 + offset,
            super::SeekType::End => inner.inode.size() as i32 + offset,
        };
        if to < 0 || to > inner.inode.size() as i32 {
            return SEEK_OUT_OF_RANGE;
        } else {
            inner.offset = to as u32;
            return to as isize;
        }
    }
}

impl OSInode {
    pub fn new(flags: OSInodeFlags, inode: Arc<Vnode>) -> Self {
        Self {
            flags,
            inner: unsafe { UPSafeCell::new(OSInodeInner { offset: 0, inode }) },
        }
    }

    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.inner.exclusive_access();
        let mut buf = [0u8; 512];
        let mut ret = Vec::new();
        loop {
            let read = inner.inode.read(inner.offset, &mut buf);
            if read == 0 {
                break;
            }
            inner.offset += read;
            ret.extend_from_slice(&buf[..read as usize]);
        }
        ret
    }

    pub fn open(name: &str, flags: OpenFlags) -> Option<Arc<Self>> {
        let inode = ROOT.dir_find(name).or_else(|| {
            if flags.contains(OpenFlags::CREATE) {
                Some(ROOT.create(name).unwrap())
            } else {
                None
            }
        })?;
        if flags.contains(OpenFlags::TRUNC) {
            inode.modify_inode(|inode| inode.clear(&YFS.data_allocator, &YFS.device));
        };
        let inode = OSInode::new(flags.into(), inode);
        if flags.contains(OpenFlags::APPEND) {
            inode.seek(SeekType::End, 0);
        }
        Some(Arc::new(inode))
    }
}

struct OSInodeInner {
    offset: u32,
    inode: Arc<Vnode>,
}

bitflags! {
    pub struct OSInodeFlags: u8 {
        const READABLE = 1 << 0;
        const WRITABLE = 1 << 1;
    }

    pub struct OpenFlags: u32 {
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const CREATE = 1 << 2;
        const APPEND = 1 << 3;
        const TRUNC = 1 << 4;
    }
}

impl From<OpenFlags> for OSInodeFlags {
    fn from(flags: OpenFlags) -> Self {
        let mut ret = OSInodeFlags::empty();
        ret.set(OSInodeFlags::READABLE, flags.contains(OpenFlags::READ));
        ret.set(OSInodeFlags::WRITABLE, flags.contains(OpenFlags::WRITE));
        ret
    }
}

lazy_static! {
    pub static ref YFS: Arc<YeFs> = YeFs::load(BLOCK_DEVICE.clone()).expect("failed to load yfs");
    pub static ref ROOT: Arc<Vnode> = YeFs::root(YFS.clone());
}
