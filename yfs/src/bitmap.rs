use core::ops::Range;

use alloc::sync::Arc;

use crate::{
    block_cache::BLOCK_CACHE,
    block_dev::BlockDevice,
    constant::{Block, BlockAddr, BLOCK_BITS},
};

pub struct Bitmap(Range<BlockAddr>);

impl Bitmap {
    pub fn new(range: Range<BlockAddr>) -> Self {
        Self(range)
    }

    pub fn alloc(&mut self, device: &Arc<dyn BlockDevice>) -> Option<usize> {
        for addr in self.0.clone() {
            let entry = { BLOCK_CACHE.lock().entry(addr, Arc::clone(device)) };
            let mut entry = entry.lock();
            if let Some((offset, pos, mut bit)) = entry
                .block()
                .iter_mut()
                .enumerate()
                .map(|entry| BitIter::new(entry))
                .flatten()
                .find(|(_, _, bit)| bit.is_unmarked())
            {
                bit.mark();
                entry.mark_dirty();
                return Some(addr as usize * BLOCK_BITS + offset * 8 + pos as usize);
            }
        }
        return None;
    }

    pub fn dealloc(
        &mut self,
        device: Arc<dyn BlockDevice>,
        (block_addr, offset, pos): (u32, usize, u8),
    ) {
        { BLOCK_CACHE.lock().entry(block_addr, device) }
            .lock()
            .modify(|block: &mut Block| {
                BitProxy::new(block.get_mut(offset).unwrap(), pos).set(false);
            })
    }
}

struct BitProxy {
    target: *mut u8,
    pos: u8,
}

impl BitProxy {
    fn new(target: &mut u8, pos: u8) -> Self {
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

    fn flip(&mut self) {
        unsafe { *self.target ^= 1 << self.pos };
    }

    fn apply(&mut self, f: impl FnOnce(bool) -> bool) {
        self.set(f(self.get()));
    }

    fn pos(&self) -> u8 {
        self.pos
    }

    fn is_marked(&self) -> bool {
        self.get()
    }

    fn is_unmarked(&self) -> bool {
        !self.get()
    }

    fn mark(&mut self) {
        self.set(true);
    }

    fn unmark(&mut self) {
        self.set(false);
    }
}

//对一个字节的位进行迭代
struct BitIter<'a> {
    target: &'a mut u8,
    pos: u8,
    offset: usize,
}

impl<'a> BitIter<'a> {
    fn new((offset, target): (usize, &'a mut u8)) -> Self {
        Self {
            target,
            pos: 0,
            offset,
        }
    }
}

impl<'a> Iterator for BitIter<'a> {
    type Item = (usize, u8, BitProxy);

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
