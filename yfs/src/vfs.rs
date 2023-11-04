use core::mem::size_of;

use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    block_cache::BLOCK_CACHE, block_dev::BlockDevice, constant::InodeAddr, layout::Inode, yfs::YeFs,
};

pub struct Vnode {
    addr: InodeAddr,
    fs: Arc<Mutex<YeFs>>,
    device: Arc<dyn BlockDevice>,
}

impl Vnode {
    pub fn new(addr: InodeAddr, fs: Arc<Mutex<YeFs>>, device: Arc<dyn BlockDevice>) -> Self {
        Self { addr, fs, device }
    }

    pub fn read_inode<V>(&self, f: impl FnOnce(&Inode) -> V) -> V {
        {
            BLOCK_CACHE
                .lock()
                .entry(self.addr.0, Arc::clone(&self.device))
        }
        .lock()
        .read_at(self.addr.1 * size_of::<Inode>() as u32, f)
    }

    pub fn modify_inode<V>(&self, f: impl FnOnce(&mut Inode) -> V) -> V {
        {
            BLOCK_CACHE
                .lock()
                .entry(self.addr.0, Arc::clone(&self.device))
        }
        .lock()
        .modify_at(self.addr.1 * size_of::<Inode>() as u32, f)
    }

    pub fn find(&self, name: &str) -> Option<Arc<Vnode>> {}
}
