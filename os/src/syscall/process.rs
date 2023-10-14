use log::{debug, info};

use crate::{task::SCHEDULER, timer::get_time_ms};

pub fn sys_exit(code: i32) -> isize {
    info!(
        "[kernel] sys_exit: process {} exited with code {}",
        SCHEDULER.exclusive_access().current(),
        code
    );
    SCHEDULER.exclusive_access().kill_current().schedule();
    0
}

pub fn sys_yield() -> isize {
    debug!(
        "[kernel] sys_yield: process {} yield",
        SCHEDULER.exclusive_access().current()
    );
    SCHEDULER.exclusive_access().suspend_current().schedule();
    0
}

pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}
