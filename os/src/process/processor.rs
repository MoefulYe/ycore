use crate::sync::up::UPSafeCell;
use crate::trap::context::Context as TrapContext;

use super::context::Context as TaskContext;

use super::pcb::ProcessControlBlock;

pub struct Processor {
    current: *mut ProcessControlBlock,
    idle_task_ctx: TaskContext,
}

unsafe impl Send for Processor {}
unsafe impl Sync for Processor {}

impl Processor {
    pub fn new() -> Self {
        Self {
            current: core::ptr::null_mut(),
            idle_task_ctx: TaskContext::idle(),
        }
    }

    pub fn take(&mut self) -> *mut ProcessControlBlock {
        let ret = self.current;
        self.current = core::ptr::null_mut();
        ret
    }

    pub fn current(&mut self) -> Option<&mut ProcessControlBlock> {
        if self.current == core::ptr::null_mut() {
            None
        } else {
            unsafe { Some(&mut *self.current) }
        }
    }

    pub fn current_token(&mut self) -> Option<usize> {
        self.current().map(|c| c.token())
    }

    pub fn current_trap_ctx(&mut self) -> Option<&'static mut TrapContext> {
        self.current().map(|c| c.trap_ctx())
    }

    pub fn idle_task_ctx(&mut self) -> *mut TaskContext {
        &mut self.idle_task_ctx as *mut _
    }

    pub fn run_tasks(&mut self) {
        loop {}
    }
}

lazy_static! {
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}
