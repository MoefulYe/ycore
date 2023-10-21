use crate::{loader::Loader, sync::up::UPSafeCell};

use super::pcb::ProcessControlBlock;

lazy_static! {
    pub static ref INITPROC: UPSafeCell<ProcessControlBlock> = unsafe {
        UPSafeCell::new(ProcessControlBlock::initproc(
            Loader::get_app_data_by_name("initproc").unwrap(),
        ))
    };
}
