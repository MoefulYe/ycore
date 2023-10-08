pub mod context;
pub mod switch;
pub mod tcb;

use crate::{
    constant::MAX_APP_NUM,
    loader::init_app_cx,
    sbi::shutdown,
    task::{switch::__switch, tcb::State},
};
use log::{info, warn};
use tcb::TaskControlBlock;

use self::context::Context;

#[derive(Default)]
pub struct Scheduler {
    num_app: usize,
    current_app: usize,
    tasks: [TaskControlBlock; MAX_APP_NUM],
}

impl Scheduler {
    pub fn init(num_app: usize) {
        info!("[kernel: scheduler] init scheduler");
        *Self::singletion() = Self::new(num_app);
    }

    pub fn singletion() -> &'static mut Scheduler {
        static mut SCHEDULER: Scheduler = Scheduler {
            num_app: 0,
            current_app: 0,
            tasks: [TaskControlBlock {
                context: Context {
                    s_regs: [0; 12],
                    ra: 0,
                    sp: 0,
                },
                state: State::Uninit,
            }; MAX_APP_NUM],
        };
        unsafe { &mut SCHEDULER }
    }

    pub fn current(&self) -> usize {
        self.current_app
    }

    pub fn run(&self) {
        info!("[kernel: scheduler] run app {}", self.current_app);
        let _unused = &mut Context::new();
        unsafe {
            __switch(_unused, &self.tasks[0].context);
        }
        unreachable!()
    }

    pub fn suspend_current(&mut self) -> &mut Self {
        info!("[kernel: scheduler] suspend app {}", self.current_app);
        let current = self.current_app;
        self.tasks.get_mut(current).unwrap().state = State::Ready;
        self
    }

    pub fn kill_current(&mut self) -> &mut Self {
        info!("[kernel: scheduler] kill app {}", self.current_app);
        let current = self.current_app;
        self.tasks.get_mut(current).unwrap().state = State::Exited;
        self
    }

    pub fn schedule(&mut self) {
        if let Some(next) = self.find_next() {
            info!(
                "[kernel: scheduler] schedule task {} -> {}",
                self.current_app, next
            );
            let cur = self.current_app;
            self.current_app = next;
            let cur = &mut self.tasks[cur].context as *mut Context;
            let next = &self.tasks[next].context as *const Context;
            unsafe {
                __switch(cur, next);
            }
        } else {
            warn!("[kernel: scheduler] all tasks completed! shut down...");
            shutdown(false)
        }
    }

    fn new(num_app: usize) -> Scheduler {
        let mut tasks = [TaskControlBlock::default(); MAX_APP_NUM];
        for (app_id, task) in tasks.iter_mut().enumerate() {
            task.context = Context::goto_restore(init_app_cx(app_id));
            task.state = State::Ready;
        }
        Scheduler {
            num_app,
            current_app: 0,
            tasks,
        }
    }

    fn find_next(&mut self) -> Option<usize> {
        (self.current_app + 1..self.current_app + 1 + self.num_app)
            .map(|i| i % self.num_app)
            .find(|i| self.tasks[*i].state == State::Ready)
    }
}
