use log::info;

use crate::task::TaskManager;

pub fn sys_exit(code: i32) -> ! {
    let manager = TaskManager::singletion();
    info!(
        "[kernel] sys_exit: process {} exited with code {}",
        manager.current(),
        code
    );
    manager.run_next();
    unreachable!()
}
