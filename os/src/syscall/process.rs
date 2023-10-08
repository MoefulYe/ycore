use log::{debug, info};

use crate::task::Scheduler;

pub fn sys_exit(code: i32) -> ! {
    let scheduler = Scheduler::singletion();
    info!(
        "[kernel] sys_exit: process {} exited with code {}",
        scheduler.current(),
        code
    );
    scheduler.kill_current().schedule()
}

pub fn sys_yield() {
    let scheduler = Scheduler::singletion();
    debug!("[kernel] sys_yield: process {} yield", scheduler.current());
    scheduler.suspend_current().schedule()
}
