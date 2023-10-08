use log::info;

use crate::{
    loader::{get_base_i, KernelStack, UserStack},
    sbi::shutdown,
    trap::context::Context,
};

pub struct TaskManager {
    num_app: usize,
    current_app: usize,
}

impl TaskManager {
    pub fn current(&self) -> usize {
        self.current_app
    }

    pub fn next(&mut self) -> bool {
        self.current_app += 1;
        self.current_app < self.num_app
    }

    fn new(num_app: usize) -> TaskManager {
        TaskManager {
            num_app,
            current_app: 0,
        }
    }

    pub fn singletion() -> &'static mut TaskManager {
        static mut TASK_MANAGER: TaskManager = TaskManager {
            num_app: 0,
            current_app: 0,
        };
        unsafe { &mut TASK_MANAGER }
    }

    pub fn init(num_app: usize) {
        info!("[kernel] Init TaskManager");
        *Self::singletion() = Self::new(num_app);
    }

    pub fn run(&self) {
        info!("[kernel] Run app {}", self.current_app);
        extern "C" {
            fn __restore(cx_addr: usize);
        }

        unsafe {
            let init_context = Context::new(
                get_base_i(self.num_app),
                UserStack::singleton()[self.num_app].get_sp(),
            );
            let cx_ptr = KernelStack::singleton()[self.num_app].push_context(init_context) as usize;
            __restore(cx_ptr)
        }
    }
    pub fn run_next(&mut self) {
        if self.next() {
            self.run();
        } else {
            info!("[kernel] No app to run, shutting down...");
            shutdown(false);
        }
    }
}
