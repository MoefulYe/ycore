use crate::{
    constant::{PPN_MASK, PPN_WIDTH},
    fs::inode::{OSInode, OpenFlags},
    mm::{
        address::{PhysPageNum, UserBuffer, VirtAddr},
        page_table::TopLevelEntry,
    },
    process::processor::PROCESSOR,
    sbi::console_getchar,
};
use log::info;

pub const STDIN: usize = 0;
pub const STDOUT: usize = 1;
pub const STDERR: usize = 2;

pub fn sys_write(fd: usize, buf: usize, len: usize) -> isize {
    match fd {
        STDOUT => {
            let token = PROCESSOR.exclusive_access().current_token().unwrap();
            let entry = PhysPageNum::from(token & PPN_MASK);
            let write_start = VirtAddr(buf);
            let write_end = write_start + len;
            let iter = UserBuffer::new(write_start..write_end, TopLevelEntry(entry));
            for buf in iter {
                print!("{}", core::str::from_utf8(buf).unwrap());
            }
            len as isize
        }
        _ => panic!("sys_write: fd {fd} not supported!"),
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        STDIN => {
            assert!(len == 1, "sys_read: len must be 1");
            let c = console_getchar() as u8;
            *TopLevelEntry::from_token(PROCESSOR.exclusive_access().current_token().unwrap())
                .translate_virt_ptr(buf as *mut u8) = c;
            1
        }
        _ => panic!("sys_read: fd {fd} not supported!"),
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let pcb = PROCESSOR.exclusive_access().current().unwrap();
    let path = TopLevelEntry::from_token(pcb.token()).translate_virt_str(path);
    if let Some(inode) = OSInode::open(&path, OpenFlags::from_bits(flags).unwrap()) {
        pcb.add_fd(inode) as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let pcb = PROCESSOR.exclusive_access().current().unwrap();
    pcb.close_fd(fd)
}
