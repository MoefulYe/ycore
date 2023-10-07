use log::info;

use crate::batch::AppManager;

pub fn sys_exit(code: i32) -> ! {
    let manager = AppManager::singleton();
    info!(
        "sys_exit: process {} exited with code {}",
        manager.current(),
        code
    );
    manager.load_next().run_app();
    unreachable!()
}
