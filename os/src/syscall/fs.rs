

use crate::{
    fs::{
        inode::{OSInode, OpenFlags},
        pipe::make_pipe,
        SeekType,
    },
    mm::address::{UserBuffer, VirtAddr},
    process::processor::PROCESSOR,
    types::CStr,
};

pub fn sys_dup(fd: usize) -> isize {
    let task = PROCESSOR.exclusive_access().current().unwrap();
    match task.fd_at(fd) {
        Some(file) => task.add_fd(file) as isize,
        None => -1,
    }
}

pub fn sys_write(fd: usize, buf: usize, len: usize) -> isize {
    let task = PROCESSOR.exclusive_access().current().unwrap();
    let page_table = task.page_table();
    match task.fd_at(fd) {
        Some(file) => {
            let user_buf = UserBuffer::new(VirtAddr(buf)..VirtAddr(buf + len), page_table);
            file.write(user_buf)
        }
        None => -1,
    }
}

pub fn sys_read(fd: usize, buf: usize, len: usize) -> isize {
    let task = PROCESSOR.exclusive_access().current().unwrap();
    let page_table = task.page_table();
    match task.fd_at(fd) {
        Some(file) => {
            let user_buf = UserBuffer::new(VirtAddr(buf)..VirtAddr(buf + len), page_table);
            file.read(user_buf)
        }
        None => -1,
    }
}

pub fn sys_seek(fd: usize, offset: isize, whence: usize) -> isize {
    let seek_ty = match whence {
        0 => SeekType::Set,
        1 => SeekType::Cur,
        2 => SeekType::End,
        _ => return -1,
    };
    let task = PROCESSOR.exclusive_access().current().unwrap();
    match task.fd_at(fd) {
        Some(file) => file.seek(seek_ty, offset as i32),
        None => -1,
    }
}

pub fn sys_open(path: CStr, flags: usize) -> isize {
    let pcb = PROCESSOR.exclusive_access().current().unwrap();
    let path = pcb.page_table().translate_virt_str(path);
    if let Some(inode) = OSInode::open(&path, OpenFlags::from_bits(flags as u32).unwrap()) {
        pcb.add_fd(inode) as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let pcb = PROCESSOR.exclusive_access().current().unwrap();
    pcb.close_fd(fd)
}

pub fn sys_pipe(pipe: *mut usize) -> isize {
    let pcb = PROCESSOR.exclusive_access().current().unwrap();
    let page_table = pcb.page_table();
    let (reader, writer) = make_pipe();
    let read_fd = pcb.add_fd(reader);
    let write_fd = pcb.add_fd(writer);
    *page_table.translate_virt_mut(pipe) = read_fd;
    *page_table.translate_virt_mut(unsafe { pipe.add(1) }) = write_fd;
    0
}
