#![allow(unused)]
pub mod context;
pub mod switch;
pub mod tcb;

use crate::{
    loader::Loader,
    sbi::shutdown,
    sync::up::UPSafeCell,
    task::{switch::__switch, tcb::State},
    timer, trap,
};
use alloc::vec::Vec;
use log::{info, warn};
use tcb::TaskControlBlock;

use self::context::Context;

#[derive(Default)]
pub struct Scheduler {
    current_app: usize,
    tasks: Vec<TaskControlBlock>,
}

impl Scheduler {
    pub fn current(&self) -> usize {
        self.current_app
    }

    pub fn run(&mut self) {
        info!("[scheduler] run app {}", self.current_app);
        let _unused = &mut Context::new();
        self.tasks[0].state = State::Running;
        timer::set_next_trigger();
        unsafe {
            __switch(_unused, &self.tasks[0].context);
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
            let cur = &mut self.tasks[cur].context as *mut Context;
            let next = &self.tasks[next].context as *const Context;
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
        self.tasks[self.current_app].get_trap_ctx()
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
        current.state = State::Exited;
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
            tasks.push(TaskControlBlock::new(Loader::nth_app_data(i), i));
        }
        UPSafeCell::new(Scheduler {
            current_app: 0,
            tasks,
        })
    };
}
