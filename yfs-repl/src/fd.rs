use anyhow::{anyhow, Ok};
use bitflags::bitflags;
use std::sync::Arc;
use yfs::{layout::DirEntry, vfs::Vnode};

use crate::{error::Result, repl::Repl};

const MAX_TABLE_SIZE: usize = 8;

struct Entry {
    pub vnode: Arc<Vnode>,
    pub readable: bool,
    pub writable: bool,
    pub offset: u32,
}

#[derive(Default)]
pub struct Table([Option<Entry>; MAX_TABLE_SIZE]);
pub type Fd = usize;
pub const NULL: Fd = usize::MAX;

impl Table {
    pub fn new() -> Self {
        Default::default()
    }

    fn find_empty(&mut self) -> Option<(Fd, &mut Option<Entry>)> {
        self.0
            .iter_mut()
            .enumerate()
            .find(|(_, entry)| entry.is_none())
    }

    fn at(&mut self, fd: Fd) -> Option<&mut Option<Entry>> {
        self.0.get_mut(fd)
    }

    pub fn open(&mut self, path: &str, flags: OpenFlags, ctx: &mut Repl) -> Result<Fd> {
        let (mut cur, path) = if path.starts_with('/') {
            (ctx.root(), &path[1..])
        } else {
            (ctx.cwd(), path)
        };

        let vnode = if flags.contains(OpenFlags::CREATE) {
            Self::find_may_create(path, cur, ctx)?
        } else {
            Self::find(path, cur, ctx)?
        };

        // 清除文件内容
        if flags.contains(OpenFlags::TRUNC) {
            vnode.clear();
        }

        let offset = if flags.contains(OpenFlags::APPEND) {
            vnode.size()
        } else {
            0
        };

        let entry = Entry {
            vnode,
            readable: flags.contains(OpenFlags::READ),
            writable: flags.contains(OpenFlags::WRITE),
            offset,
        };

        self.find_empty()
            .map(|(fd, _entry)| {
                *_entry = Some(entry);
                fd
            })
            .ok_or_else(|| anyhow!("too many files opened"))
    }

    pub fn close(&mut self, fd: Fd) -> Result<()> {
        match self.at(fd) {
            Some(entry) => {
                if entry.is_none() {
                    Err(anyhow!("invalid fd"))?
                } else {
                    *entry = None;
                    Ok(())
                }
            }
            None => Err(anyhow!("invalid fd"))?,
        }
    }

    pub fn read(&mut self, fd: Fd, buf: &mut [u8]) -> Result<u32> {
        match self.at(fd) {
            Some(Some(entry)) => {
                if !entry.readable {
                    return Err(anyhow!("file not readable"));
                }
                let read = entry.vnode.read(entry.offset, buf);
                entry.offset += read;
                Ok(read)
            }
            _ => Err(anyhow!("invalid fd"))?,
        }
    }

    pub fn write(&mut self, fd: Fd, buf: &[u8]) -> Result<u32> {
        match self.at(fd) {
            Some(Some(entry)) => {
                if !entry.writable {
                    return Err(anyhow!("file not writable"));
                }
                let write = entry.vnode.write(entry.offset, buf);
                entry.offset += write;
                Ok(write)
            }
            _ => Err(anyhow!("invalid fd"))?,
        }
    }

    pub fn seek(&mut self, fd: Fd, step: i32) -> Result<()> {
        match self.at(fd) {
            Some(Some(entry)) => {
                entry.offset = if step < 0 {
                    entry
                        .offset
                        .checked_sub(step.abs() as u32)
                        .ok_or_else(|| anyhow!("seek before start"))?
                } else {
                    entry.offset + step as u32
                };
                Ok(())
            }
            _ => Err(anyhow!("invalid fd"))?,
        }
    }

    fn find(path: &str, mut cur: Arc<Vnode>, ctx: &mut Repl) -> Result<Arc<Vnode>> {
        for name in path.split("/") {
            if name.is_empty() {
                continue;
            }
            match cur.dir_find(name) {
                Some(vnode) => cur = vnode,
                None => Err(anyhow!("no such file or directory"))?,
            }
        }
        Ok(cur)
    }

    fn find_may_create(path: &str, mut cur: Arc<Vnode>, ctx: &mut Repl) -> Result<Arc<Vnode>> {
        let mut iter = path.split("/");
        loop {
            if let Some(name) = iter.next() {
                if name.is_empty() {
                    continue;
                }
                match cur.dir_find(name) {
                    Some(vnode) => cur = vnode,
                    None => {
                        if let Some(_) = iter.clone().next() {
                            Err(anyhow!("no such file or directory"))?;
                        } else {
                            return Ok(cur.create(name).unwrap());
                        }
                    }
                }
            } else {
                break;
            }
        }
        Ok(cur)
    }
}

bitflags! {
    pub struct OpenFlags: u32 {
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const CREATE = 1 << 2;
        const APPEND = 1 << 3;
        const TRUNC = 1 << 4;
    }
}
