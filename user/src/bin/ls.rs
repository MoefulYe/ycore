#![no_std]
#![no_main]
extern crate alloc;

use alloc::vec::Vec;
use ylib::{fclose, fopen, fread, println, types::Argv, OpenFlags};

const NAME_LEN_LIMIT: usize = 26;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct DirEntry {
    valid: bool,
    name: [u8; NAME_LEN_LIMIT + 1],
    inode_idx: u32,
}

impl DirEntry {
    fn name(&self) -> &str {
        let len = self
            .name
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(NAME_LEN_LIMIT);
        unsafe { core::str::from_utf8_unchecked(&self.name[..len]) }
    }
}

fn as_entries<'a>(slice: &'a [u8]) -> &'a [DirEntry] {
    unsafe {
        core::slice::from_raw_parts(
            slice.as_ptr() as *const DirEntry,
            slice.len() / core::mem::size_of::<DirEntry>(),
        )
    }
}

#[no_mangle]
fn main(_: &Argv) -> i32 {
    let fd = fopen(".\0".as_ptr(), OpenFlags::READ).expect("ls: open failed");
    let mut buf = [0u8; 128];
    let mut bytes = Vec::new();
    loop {
        let read = fread(fd, &mut buf).expect("ls: read failed");
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buf[..read]);
    }

    let total = as_entries(&bytes)
        .iter()
        .filter(|entry| entry.valid)
        .fold(0, |cnt, entry| {
            let name = entry.name();
            let inode = entry.inode_idx;
            println!("{}: {}", name, inode);
            cnt + 1
        });

    println!("\ntotal: {}", total);

    fclose(fd).expect("ls: close failed");
    0
}
