use alloc::sync::Arc;

use crate::{
    block_cache::cache_entry,
    block_dev::BlockDevice,
    constant::{Block, BlockAddr, BLOCK_BITS},
};

pub struct Bitmap {
    bitmap_start: BlockAddr,
    bitmap_size: u32,
    device: Arc<dyn BlockDevice>,
}

impl Bitmap {
    pub fn new(start: BlockAddr, size: u32, device: Arc<dyn BlockDevice>) -> Self {
        Self {
            bitmap_start: start,
            bitmap_size: size,
            device,
        }
    }

    pub fn alloc(&mut self) -> u32 {
        for block_idx in 0..self.bitmap_size {
            let entry = cache_entry(self.bitmap_start + block_idx, self.device.clone());
            let mut entry = entry.lock();
            if let Some((offset, pos, mut bit)) = entry
                .block()
                .iter_mut()
                .enumerate()
                .flat_map(|(offset, byte)| BitIter::new(offset as u32, byte))
                .find(|(_, _, bit)| bit.is_unmarked())
            {
                bit.mark();
                entry.mark_dirty();
                return Self::detriple((block_idx, offset, pos));
            }
        }
        panic!("No space left!");
    }

    pub fn dealloc(&mut self, num: u32) {
        let (block_idx, offset, pos) = Self::triple(num);
        cache_entry(self.bitmap_start + block_idx, self.device.clone())
            .lock()
            .modify(|block: &mut Block| {
                BitProxy::new(block.get_mut(offset as usize).unwrap(), pos).set(false)
            })
    }

    fn triple(num: u32) -> (u32, u32, u32) {
        let block_addr = num / BLOCK_BITS as u32;
        let offset = (num % BLOCK_BITS as u32) / 8;
        let pos = (num % BLOCK_BITS as u32) % 8;
        (block_addr, offset, pos)
    }

    fn detriple((num, offset, pos): (u32, u32, u32)) -> u32 {
        num * BLOCK_BITS as u32 + offset * 8 + pos
    }

    pub fn bit_size(&self) -> u32 {
        self.bitmap_size * BLOCK_BITS as u32
    }
}

struct BitProxy {
    target: *mut u8,
    pos: u32,
}

impl BitProxy {
    fn new(target: &mut u8, pos: u32) -> Self {
        Self {
            target: target as *mut _,
            pos,
        }
    }

    fn get(&self) -> bool {
        unsafe { *self.target & (1 << self.pos) != 0 }
    }

    fn set(&mut self, value: bool) {
        unsafe {
            if value {
                *self.target |= 1 << self.pos;
            } else {
                *self.target &= !(1 << self.pos);
            }
        }
    }

    #[allow(unused)]
    fn flip(&mut self) {
        unsafe { *self.target ^= 1 << self.pos };
    }

    #[allow(unused)]
    fn apply(&mut self, f: impl FnOnce(bool) -> bool) {
        self.set(f(self.get()));
    }

    #[allow(unused)]
    fn pos(&self) -> u32 {
        self.pos
    }

    #[allow(unused)]
    fn is_marked(&self) -> bool {
        self.get()
    }

    fn is_unmarked(&self) -> bool {
        !self.get()
    }

    fn mark(&mut self) {
        self.set(true);
    }

    #[allow(unused)]
    fn unmark(&mut self) {
        self.set(false);
    }
}

//对一个字节的位进行迭代
struct BitIter<'a> {
    target: &'a mut u8,
    pos: u32,
    // 块内偏移
    offset: u32,
}

impl<'a> BitIter<'a> {
    fn new(offset: u32, target: &'a mut u8) -> Self {
        Self {
            target,
            pos: 0,
            offset,
        }
    }
}

impl<'a> Iterator for BitIter<'a> {
    type Item = (u32, u32, BitProxy);

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < 8 {
            let ret = (self.offset, self.pos, BitProxy::new(self.target, self.pos));
            self.pos += 1;
            Some(ret)
        } else {
            None
        }
    }
}
