use crate::{
    fs::inode::{OSInode, OpenFlags},
    sync::up::UPSafeCell,
};

use super::pcb::ProcessControlBlock;

lazy_static! {
    pub static ref INITPROC: UPSafeCell<ProcessControlBlock> = unsafe {
        let data = OSInode::open("initproc", OpenFlags::READ)
            .unwrap()
            .read_all();
        UPSafeCell::new(ProcessControlBlock::initproc(&data))
    };
}
