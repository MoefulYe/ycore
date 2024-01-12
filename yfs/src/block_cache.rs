extern crate alloc;
use crate::block_dev::BlockDevice;
use crate::constant::*;
use alloc::{sync::Arc, vec::Vec};
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

/// mut后缀的方法只是暗示缓存项会被标记成脏项, 无论mut与否得到的数据都是可变引用
/// 如果要更改需要显式地调用mark_dirty来告诉缓存该缓存项已经被修改, 回收时需要落盘
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

    pub fn block(&mut self) -> &mut Block {
        self.mark_access();
        &mut self.data
    }

    pub fn block_mut(&mut self) -> &mut Block {
        self.mark_access();
        self.mark_dirty();
        &mut self.data
    }

    pub fn data<T>(&mut self) -> &mut T
    where
        T: Sized,
    {
        assert!(
            size_of::<T>() <= BLOCK_SIZE,
            "the data must be limited in the block"
        );
        self.mark_access();
        unsafe { &mut *(self.data.as_ptr() as usize as *mut T) }
    }

    pub fn data_mut<T>(&mut self) -> &mut T
    where
        T: Sized,
    {
        assert!(
            size_of::<T>() <= BLOCK_SIZE,
            "the data must be limited in the block"
        );
        self.mark_dirty();
        self.mark_access();
        unsafe { &mut *(self.data.as_ptr() as usize as *mut T) }
    }

    pub fn new(device: Arc<dyn BlockDevice>, addr: BlockAddr) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::_new(device, addr)))
    }

    fn addr_at(&self, offset: u32) -> usize {
        &self.data[offset as usize] as *const _ as usize
    }

    pub fn at<T>(&mut self, offset: u32) -> &mut T
    where
        T: Sized,
    {
        assert!(
            size_of::<T>() + offset as usize <= BLOCK_SIZE,
            "the data must be limited in the block"
        );
        self.mark_access();
        unsafe { &mut *(self.addr_at(offset) as *mut T) }
    }

    pub fn at_mut<T>(&mut self, offset: u32) -> &mut T {
        assert!(
            size_of::<T>() + offset as usize <= BLOCK_SIZE,
            "the data must be limited in the block"
        );
        self.mark_access();
        self.mark_dirty();
        unsafe { &mut *(self.addr_at(offset) as *mut T) }
    }

    pub fn read<T, V>(&mut self, f: impl FnOnce(&T) -> V) -> V {
        f(self.data())
    }

    pub fn modify<T, V>(&mut self, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.data_mut())
    }

    pub fn read_at<T, V>(&mut self, offset: u32, f: impl FnOnce(&T) -> V) -> V {
        f(self.at(offset))
    }

    pub fn modify_at<T, V>(&mut self, offset: u32, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.at_mut(offset))
    }

    pub fn sync(&mut self) {
        if self.dirty {
            self.dirty = false;
            self.device.write_block(self.addr, &self.data);
        }
        self.access = false;
    }

    pub fn clear(&mut self) {
        self.mark_dirty();
        self.mark_access();
        self.data = [0u8; BLOCK_SIZE];
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn mark_access(&mut self) {
        self.access = true;
    }
}

impl Drop for CacheEntry {
    fn drop(&mut self) {
        self.sync();
    }
}

struct BlockCache(Vec<(BlockAddr, Arc<Mutex<CacheEntry>>)>);

impl BlockCache {
    const BLOCK_CACHE_SIZE: usize = 16;

    fn new() -> Mutex<Self> {
        Mutex::new(Self(Vec::new()))
    }

    fn entry(&mut self, addr: BlockAddr, device: Arc<dyn BlockDevice>) -> Arc<Mutex<CacheEntry>> {
        if let Some((_, ref entry)) = self.0.iter().find(|item| item.0 == addr) {
            //如果存在缓存项
            Arc::clone(entry)
        } else if self.0.len() < Self::BLOCK_CACHE_SIZE {
            //如果缓存项未满
            let new_entry = CacheEntry::new(device, addr);
            let entry = Arc::clone(&new_entry);
            self.0.push((addr, new_entry));
            entry
        } else {
            let entry = CacheEntry::new(device, addr);
            self.replace((addr, entry.clone()));
            entry
        }
    }

    // 缓存替换策略
    // 返回新的缓存项的引用
    // 旧的缓存项会被回收
    fn replace(&mut self, new_entry: (BlockAddr, Arc<Mutex<CacheEntry>>)) {
        let mut iter = self
            .0
            .iter_mut()
            .filter(|entry| Arc::strong_count(&entry.1) == 1);
        //TODO: 未考虑到缓存项的access和dirty情况, 替换策略还不是很合理
        if let Some(entry) = iter.next() {
            *entry = new_entry;
        } else {
            panic!("no cache entry can be replaced")
        }
    }

    fn flush(&self) {
        self.0.iter().for_each(|(_, entry)| entry.lock().sync())
    }
}

lazy_static! {
    static ref BLOCK_CACHE: Mutex<BlockCache> = BlockCache::new();
}

pub fn cache_entry(addr: BlockAddr, device: Arc<dyn BlockDevice>) -> Arc<Mutex<CacheEntry>> {
    BLOCK_CACHE.lock().entry(addr, device)
}

pub fn flush() {
    BLOCK_CACHE.lock().flush()
}
