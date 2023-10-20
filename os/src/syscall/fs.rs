#![allow(unused)]
use crate::{
    constant::{PPN_MASK, PPN_WIDTH},
    mm::{
        address::{PhysPageNum, VirtAddr, VirtBufIter},
        page_table::TopLevelEntry,
    },
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
            let write_start = VirtAddr(buf);
            let write_end = write_start + len;
            let iter = VirtBufIter::new(write_start..write_end, TopLevelEntry(entry));
            for buf in iter {
                print!("{}", core::str::from_utf8(buf).unwrap());
            }
            len as isize
        }
        _ => panic!("sys_write: fd {fd} not supported!"),
    }
}
