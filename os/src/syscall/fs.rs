#![allow(unused)]
use crate::{
    constant::{PPN_MASK, PPN_WIDTH},
    mm::address::{PhysPageNum, VirtAddr, VirtBufIter},
    process::SCHEDULER,
};
use log::{debug, info};

pub const STDIN: usize = 0;
pub const STDOUT: usize = 1;
pub const STDERR: usize = 2;

pub fn sys_write(fd: usize, buf: usize, len: usize) -> isize {
    debug!("sys_write: fd: {}, buffer: {:#x}, len: {:#x}", fd, buf, len);
    match fd {
        STDOUT => {
            let token = SCHEDULER.exclusive_access().get_current_token();
            let entry = PhysPageNum::from(token & PPN_MASK);
            for buf in VirtBufIter::new(entry, VirtAddr(buf), len) {
                print!("{}", core::str::from_utf8(buf).unwrap());
            }
            len as isize
        }
        _ => panic!("sys_write: fd {fd} not supported!"),
    }
}
