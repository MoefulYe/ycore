use super::address::PhysPageNum;
use crate::{constant::MEMORY_END, mm::address::PhysAddr, sync::up::UPSafeCell};
use alloc::collections::VecDeque;

pub struct FrameAllocator {
    pool: VecDeque<usize>,
}

lazy_static! {
    pub static ref ALLOCATOR: UPSafeCell<FrameAllocator> = unsafe {
        extern "C" {
            fn ekernel();
        }
        let start = PhysAddr(ekernel as usize).phys_page_num();
        let end = PhysAddr(MEMORY_END).phys_page_num();
        let inner = FrameAllocator::new(start, end);
        UPSafeCell::new(inner)
    };
}

impl FrameAllocator {
    fn new(PhysPageNum(l): PhysPageNum, PhysPageNum(r): PhysPageNum) -> Self {
        Self {
            pool: (l..r).collect(),
        }
    }

    pub fn try_alloc(&mut self) -> Option<PhysPageNum> {
        self.pool.pop_front().map(|ppn| PhysPageNum(ppn).clear())
    }
    pub fn alloc(&mut self) -> PhysPageNum {
        match self.try_alloc() {
            Some(ppn) => ppn,
            None => panic!("out of memory"),
        }
    }

    pub fn dealloc(&mut self, PhysPageNum(p): PhysPageNum) {
        if self.pool.iter().find(|&item| *item == p).is_some() {
            panic!("dealloc a frame twice");
        } else {
            self.pool.push_back(p);
        }
    }
}
