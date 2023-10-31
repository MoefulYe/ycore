use core::ops::Range;

use alloc::sync::Arc;

use crate::{block_cache::BLOCK_CACHE, block_dev::BlockDevice, constant::BlockAddr};

pub struct Bitmap(Range<BlockAddr>);

impl Bitmap {
    pub fn new(range: Range<BlockAddr>) -> Self {
        Self(range)
    }

    pub fn iter(&self, device: Arc<dyn BlockDevice>) -> impl Iterator<Item = BitProxy> {
        self.0
            .clone()
            .into_iter()
            .map(|addr| {
                BLOCK_CACHE
                    .lock()
                    .get_cache(addr, Arc::clone(&device))
                    .lock()
                    .data_mut()
                    .iter_mut()
            })
            .flatten()
            .map(|byte| U8Iter::new(byte, 0))
            .flatten()
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
}

struct U8Iter<'a> {
    target: &'a mut u8,
    pos: usize,
}

impl<'a> U8Iter<'a> {
    fn new(target: &'a mut u8, pos: usize) -> Self {
        Self { target, pos }
    }
}

impl<'a> Iterator for U8Iter<'a> {
    type Item = BitProxy;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < 8 {
            let proxy = BitProxy::new(self.target, self.pos as u8);
            self.pos += 1;
            Some(proxy)
        } else {
            None
        }
    }
}
