use alloc::sync::Arc;

use crate::{
    bitmap::Bitmap,
    block_dev::BlockDevice,
    constant::{addr2inode, inode2addr, BlockAddr, InodeAddr},
};
use spin::Mutex;

#[derive(Debug)]
pub struct InodeAllocator {
    bitmap: Mutex<Bitmap>,
    data_area_start: BlockAddr,
    size: u32,
}

impl InodeAllocator {
    pub fn new(bitmap_start: BlockAddr, bitmap_size: u32, device: Arc<dyn BlockDevice>) -> Self {
        let bitmap = Bitmap::new(bitmap_start, bitmap_size, device);
        let size = bitmap.bit_size();
        Self {
            bitmap: Mutex::new(bitmap),
            data_area_start: bitmap_start + bitmap_size,
            size,
        }
    }

    pub fn alloc(&self) -> InodeAddr {
        inode2addr(self.bitmap.lock().alloc(), self.data_area_start)
    }

    pub fn dealloc(&self, addr: InodeAddr) {
        self.bitmap
            .lock()
            .dealloc(addr2inode(addr, self.data_area_start));
    }

    pub fn size(&self) -> u32 {
        self.size
    }
}

#[derive(Debug)]
pub struct DataBlockAllocator {
    bitmap: Mutex<Bitmap>,
    data_area_start: BlockAddr,
    size: u32,
}

impl DataBlockAllocator {
    pub fn new(bitmap_start: BlockAddr, bitmap_size: u32, device: Arc<dyn BlockDevice>) -> Self {
        let bitmap = Bitmap::new(bitmap_start, bitmap_size, device);
        let size = bitmap.bit_size();
        Self {
            bitmap: Mutex::new(bitmap),
            data_area_start: bitmap_start + bitmap_size,
            size,
        }
    }

    pub fn alloc(&self) -> BlockAddr {
        self.bitmap.lock().alloc() + self.data_area_start
    }

    pub fn dealloc(&self, block_addr: BlockAddr) {
        self.bitmap
            .lock()
            .dealloc(block_addr - self.data_area_start);
    }

    pub fn size(&self) -> u32 {
        self.size
    }
}
