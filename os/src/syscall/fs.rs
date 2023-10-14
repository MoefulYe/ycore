use crate::{
    constant::PPN_WIDTH,
    mm::address::{PhysPageNum, VirtBufIter},
    task::SCHEDULER,
};
use log::debug;

pub const STDIN: usize = 0;
pub const STDOUT: usize = 1;
pub const STDERR: usize = 2;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    debug!("sys_write: fd: {}, buffer: {:p}, len: {:#x}", fd, buf, len);
    match fd {
        STDOUT => {
            let token = SCHEDULER.exclusive_access().get_current_token();
            let entry = PhysPageNum::from(token & (1 << PPN_WIDTH - 1));
            for buf in VirtBufIter::new(entry, buf, len) {
                print!("{}", core::str::from_utf8(buf).unwrap());
            }
            len as isize
        }
        _ => panic!("sys_write: fd {fd} not supported!"),
    }
}
