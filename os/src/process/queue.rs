use alloc::collections::VecDeque;

use crate::sync::up::UPSafeCell;

use super::pcb::ProcessControlBlock;

pub struct Queue {
    queue: VecDeque<*mut ProcessControlBlock>,
}

unsafe impl Sync for Queue {}
unsafe impl Send for Queue {}

impl Queue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn push(&mut self, pcb: *mut ProcessControlBlock) {
        self.queue.push_back(pcb);
    }

    pub fn fetch(&mut self) -> Option<*mut ProcessControlBlock> {
        self.queue.pop_front()
    }
}

lazy_static! {
    pub static ref QUEUE: UPSafeCell<Queue> = unsafe { UPSafeCell::new(Queue::new()) };
}
