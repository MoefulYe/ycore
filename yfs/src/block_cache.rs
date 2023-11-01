extern crate alloc;
use crate::block_dev::BlockDevice;
use crate::constant::*;
use alloc::{collections::VecDeque, sync::Arc};
use core::{mem::size_of, usize};
use lazy_static::lazy_static;
use spin::Mutex;

pub struct CacheEntry {
    device: Arc<dyn BlockDevice>,
    addr: BlockAddr,
    data: Block,
    dirty: bool,
    access: bool,
}

impl CacheEntry {
    fn _new(device: Arc<dyn BlockDevice>, addr: BlockAddr) -> Self {
        let mut data = [0u8; BLOCK_SIZE];
        device.read_block(addr, &mut data);
        Self {
            device,
            addr,
            data,
            dirty: false,
            access: false,
        }
    }

    pub fn data(&self) -> &mut Block {
        self.access = true;
        &mut self.data
    }

    pub fn data_mut(&mut self) -> &mut Block {
        self.access = true;
        self.mark_dirty();
        &mut self.data
    }

    pub fn new(device: Arc<dyn BlockDevice>, addr: BlockAddr) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::_new(device, addr)))
    }

    fn addr_at(&self, offset: usize) -> usize {
        &self.data[offset] as *const _ as usize
    }

    pub fn as_<T>(&self, offset: usize) -> &mut T
    where
        T: Sized,
    {
        assert!(
            size_of::<T>() + offset <= BLOCK_SIZE,
            "the data must be limited in the block"
        );
        self.access = true;
        unsafe { &mut *(self.addr_at(offset) as *const T) }
    }

    pub fn as_mut_<T>(&mut self, offset: usize) -> &mut T {
        assert!(
            size_of::<T>() + offset <= BLOCK_SIZE,
            "the data must be limited in the block"
        );
        self.access = true;
        self.mark_dirty();
        unsafe { &mut *(self.addr_at(offset) as *mut T) }
    }

    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.as_(offset))
    }

    pub fn modify<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.as_mut_(offset))
    }

    pub fn sync(&mut self) {
        if self.dirty {
            self.dirty = false;
            self.device.write_block(self.addr, &self.data);
        }
        self.access = false;
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

impl Drop for CacheEntry {
    fn drop(&mut self) {
        self.sync()
    }
}

pub struct BlockCache(VecDeque<(BlockAddr, Arc<Mutex<CacheEntry>>)>);

impl BlockCache {
    const BLOCK_CACHE_SIZE: usize = 16;
    fn _new() -> Self {
        Self(VecDeque::new())
    }

    pub fn new() -> Mutex<Self> {
        Mutex::new(Self::_new())
    }

    pub fn get_cache(
        &mut self,
        addr: BlockAddr,
        device: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<CacheEntry>> {
        //回收策略没有考虑缓存项是否是脏块, 可能导致回收的写入频度变高
        if let Some((_, ref entry)) = self.0.iter().find(|item| item.0 == addr) {
            //如果存在缓存项
            Arc::clone(entry)
        } else if self.0.len() < Self::BLOCK_CACHE_SIZE {
            //如果缓存项未满
            let new_entry = CacheEntry::new(device, addr);
            let clone = Arc::clone(&new_entry);
            self.0.push_back((addr, new_entry));
            clone
        } else if let Some(item) = self
            .0
            .iter_mut()
            .find(|item| Arc::strong_count(&item.1) == 1)
        {
            //如果缓存项满了，但是有缓存项的引用计数为1则该缓存项没有被使用, 可以安全的替换
            let new_entry = CacheEntry::new(device, addr);
            let clone = Arc::clone(&new_entry);
            *item = (addr, new_entry);
            clone
        } else {
            //如果缓存项满了，且所有缓存项的引用计数都大于1，则panic
            panic!("run out of cache");
        }
    }

    pub fn sync(&self) {
        self.0.iter().for_each(|(_, entry)| entry.lock().sync())
    }
}

lazy_static! {
    pub static ref BLOCK_CACHE: Mutex<BlockCache> = BlockCache::new();
}
