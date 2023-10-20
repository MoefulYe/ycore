pub mod context;
pub mod pcb;
pub mod pid;
pub mod processor;
pub mod queue;
pub mod switch;

use crate::{
    loader::Loader,
    process::{pcb::State, switch::__switch},
    sbi::shutdown,
    sync::up::UPSafeCell,
    timer, trap,
};
use alloc::vec::Vec;
use log::{info, warn};
use pcb::ProcessControlBlock;

use self::context::Context;

#[derive(Default)]
pub struct Scheduler {
    current_app: usize,
    tasks: Vec<ProcessControlBlock>,
}

impl Scheduler {
    pub fn current(&self) -> usize {
        self.current_app
    }

    pub fn run(&mut self) {
        info!("[scheduler] run app {}", self.current_app);
        let _unused = &mut Context::idle();
        self.tasks[0].state = State::Running;
        timer::set_next_trigger();
        unsafe {
            __switch(_unused, &self.tasks[0].task_context);
        }
        unreachable!()
    }

    pub fn suspend_current(&mut self) -> &mut Self {
        info!("[scheduler] suspend app {}", self.current_app);
        let current = self.current_app;
        self.tasks.get_mut(current).unwrap().state = State::Ready;
        self
    }

    pub fn schedule(&mut self) {
        if let Some(next) = self.find_next() {
            info!("[scheduler] schedule task {} -> {}", self.current_app, next);
            self.tasks[next].state = State::Running;
            let cur = self.current_app;
            self.current_app = next;
            let cur = &mut self.tasks[cur].task_context as *mut Context;
            let next = &self.tasks[next].task_context as *const Context;
            timer::set_next_trigger();
            unsafe {
                __switch(cur, next);
            }
        } else {
            warn!("[scheduler] all tasks completed! shut down...");
            shutdown(false)
        }
    }

    fn find_next(&mut self) -> Option<usize> {
        (self.current_app + 1..self.current_app + 1 + self.tasks.len())
            .map(|i| i % self.tasks.len())
            .find(|i| self.tasks[*i].state == State::Ready)
    }

    pub fn get_current_token(&self) -> usize {
        self.tasks[self.current_app].mem_set.token()
    }

    pub fn get_current_trap_ctx(&self) -> &'static mut trap::context::Context {
        self.tasks[self.current_app].trap_ctx()
    }

    //改变堆顶, 成功时返回旧的堆顶, 失败时返回usize::MAX
    pub fn change_current_task_brk(&mut self, size: isize) -> usize {
        self.tasks
            .get_mut(self.current_app)
            .unwrap()
            .change_prk(size)
    }

    //回收当前进程分配的资源
    pub fn recycle_current(&mut self) -> &mut Self {
        info!("[scheduler] recycle app {}", self.current_app);
        let current = self.tasks.get_mut(self.current_app).unwrap();
        current.recycle();
        current.state = State::Zombie;
        self
    }
}

lazy_static! {
    pub static ref SCHEDULER: UPSafeCell<Scheduler> = unsafe {
        info!("[scheduler] init");
        let num_app = Loader::get_num_app();
        info!("[scheduler] {} apps found", num_app);
        let mut tasks = Vec::new();
        for i in 0..num_app {
            // tasks.push(ProcessControlBlock::initproc(Loader::nth_app_data(i), i));
        }
        UPSafeCell::new(Scheduler {
            current_app: 0,
            tasks,
        })
    };
}
