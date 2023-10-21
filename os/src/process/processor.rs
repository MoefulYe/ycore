use log::info;

use crate::process::initproc::INITPROC;
use crate::sync::up::UPSafeCell;
use crate::timer::set_next_trigger;
use crate::trap::context::Context as TrapContext;

use super::context::Context as TaskContext;

use super::pcb::{ProcessControlBlock, State};
use super::queue::QUEUE;
use super::switch::__switch;

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
        loop {
            if let Some(task) = QUEUE.exclusive_access().fetch() {
                self.current = task;
                let idle_task_ctx = self.idle_task_ctx();
                let task_ctx = self.current().unwrap().task_ctx();
                self.current().unwrap().state = State::Running;
                set_next_trigger();
                unsafe { __switch(idle_task_ctx, task_ctx) }
            }
        }
    }

    pub fn suspend_current(&mut self) -> &mut Self {
        self.current().unwrap().state = State::Ready;
        info!(
            "[processor] process {} suspend",
            self.current().unwrap().pid()
        );
        QUEUE.exclusive_access().push(self.current);
        self
    }

    pub fn exit_current(&mut self, code: i32) -> &mut Self {
        let cur = self.current().unwrap();
        info!("[processor] process {} exit with {}", cur.pid(), code);
        cur.state = State::Zombie;
        cur.exit_code = code;
        for &child in cur.children.iter() {
            unsafe {
                (*child).parent = INITPROC.exclusive_access() as *mut _;
                INITPROC.exclusive_access().children.push(child);
            }
        }
        cur.children.clear();
        cur.mem_set.recycle();
        self
    }

    pub fn schedule(&mut self) {
        let idle_task_ctx = self.idle_task_ctx();
        let switch_task_ctx = self.current().unwrap().task_ctx();
        unsafe { __switch(switch_task_ctx, idle_task_ctx) }
    }
}

lazy_static! {
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}
