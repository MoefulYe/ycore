use alloc::sync::Arc;

use crate::{
    bitmap::Bitmap,
    block_dev::BlockDevice,
    constant::{BlockAddr, InodeAddr},
};

pub trait DataBlockAlloc {
    fn alloc(&mut self) -> BlockAddr;
    fn dealloc(&mut self, block_addr: BlockAddr);
}

pub trait InodeBlockAlloc {
    fn alloc(&mut self) -> InodeAddr;
    fn dealloc(&mut self, block_addr: InodeAddr);
}

pub struct InodeBitmap {
    bitmap: Bitmap,
    data_area_start: BlockAddr,
}

impl InodeBitmap {
    pub fn new(bitmap_start: BlockAddr, bitmap_size: u32, device: Arc<dyn BlockDevice>) -> Self {
        Self {
            bitmap: Bitmap::new(bitmap_start, bitmap_size, device),
            data_area_start: bitmap_start + bitmap_size,
        }
    }

    pub fn size(&self) -> u32 {
        self.bitmap.bit_size()
    }
}

impl InodeBlockAlloc for InodeBitmap {
    fn alloc(&mut self) -> InodeAddr {
        let idx = self.bitmap.alloc().unwrap();
        (idx >> 2 + self.data_area_start, idx & 0b11)
    }

    fn dealloc(&mut self, (block_addr, offset): InodeAddr) {
        let idx = (block_addr - self.data_area_start) << 2 + offset;
        self.bitmap.dealloc(idx);
    }
}

pub struct DataBitmap {
    bitmap: Bitmap,
    data_area_start: BlockAddr,
}

impl DataBitmap {
    pub fn new(bitmap_start: BlockAddr, bitmap_size: u32, device: Arc<dyn BlockDevice>) -> Self {
        Self {
            bitmap: Bitmap::new(bitmap_start, bitmap_size, device),
            data_area_start: bitmap_start + bitmap_size,
        }
    }
    fn size(&self) -> u32 {
        self.bitmap.bit_size()
    }
}

impl DataBlockAlloc for DataBitmap {
    fn alloc(&mut self) -> BlockAddr {
        self.bitmap.alloc().unwrap() + self.data_area_start
    }

    fn dealloc(&mut self, block_addr: BlockAddr) {
        self.bitmap.dealloc(block_addr - self.data_area_start);
    }
}
