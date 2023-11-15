use alloc::{sync::Arc, vec::Vec};
use core::{mem::size_of, ops::DerefMut};

use crate::{
    block_alloc::{DataBlockAlloc, InodeAlloc},
    block_cache::BLOCK_CACHE,
    block_dev::BlockDevice,
    constant::{addr2inode, inode2addr, BlockAddr, InodeAddr},
    layout::{DirEntry, Inode, InodeType},
    yfs::YeFs,
};

#[derive(Debug)]
pub struct Vnode {
    addr: InodeAddr,
    fs: Arc<YeFs>,
    device: Arc<dyn BlockDevice>,
}

impl Vnode {
    pub fn new(addr: InodeAddr, fs: Arc<YeFs>, device: Arc<dyn BlockDevice>) -> Arc<Self> {
        Arc::new(Self { addr, fs, device })
    }

    pub fn inode_idx(&self) -> u32 {
        addr2inode(self.addr, self.fs.inode_start)
    }

    pub fn read_inode<V>(&self, f: impl FnOnce(&Inode) -> V) -> V {
        let entry = {
            BLOCK_CACHE
                .lock()
                .entry(self.addr.0, Arc::clone(&self.device))
        };
        let v = entry
            .lock()
            .read_at(self.addr.1 * size_of::<Inode>() as u32, f);
        v
    }

    pub fn modify_inode<V>(&self, f: impl FnOnce(&mut Inode) -> V) -> V {
        let entry = {
            BLOCK_CACHE
                .lock()
                .entry(self.addr.0, Arc::clone(&self.device))
        };
        let v = entry
            .lock()
            .modify_at(self.addr.1 * size_of::<Inode>() as u32, f);
        v
    }

    pub fn dir_find(&self, name: &str) -> Option<Arc<Vnode>> {
        self.read_inode(|inode| {
            inode.dir_find(name, &self.device).map(|entry| {
                Vnode::new(
                    inode2addr(entry.inode_idx, self.fs.inode_start),
                    self.fs.clone(),
                    self.device.clone(),
                )
            })
        })
    }

    pub fn dir_insert(&self, entry: DirEntry) {
        let need = self
            .read_inode(|inode| inode.new_needed_blocks(inode.size + size_of::<DirEntry>() as u32));
        let alloc = {
            let mut allocator = self.fs.data_alloc.lock();
            (0..need).fold(Vec::new(), |mut acc, _| {
                acc.push(allocator.alloc());
                acc
            })
        };
        self.modify_inode(|inode| inode.dir_insert(entry, &self.device, alloc))
    }

    pub fn dir_rm(&self, name: &str) -> Result<(), ()> {
        match self.modify_inode(|inode| inode.dir_delete(name, &self.device)) {
            Ok(entry) => {
                let inode = entry.inode_idx;
                let addr = inode2addr(inode, self.fs.inode_start);
                let to_delete = Self::new(addr, self.fs.clone(), self.device.clone());
                let dealloc = to_delete.modify_inode(|inode| inode.clear(&to_delete.device));
                {
                    let mut allocator = self.fs.data_alloc.lock();
                    dealloc.into_iter().for_each(|addr| allocator.dealloc(addr));
                }
                self.fs.inode_alloc.lock().dealloc(addr);
                return Ok(());
            }
            Err(_) => Err(()),
        }
    }

    pub fn ls(&self) -> Vec<DirEntry> {
        self.read_inode(|inode| {
            (&mut unsafe { *(inode as *const _ as usize as *mut Inode) }).dir(&self.device)
        })
    }

    pub fn mkdir(&self, name: &str) -> Result<Arc<Vnode>, Arc<Vnode>> {
        if let Some(vnode) = self.dir_find(name) {
            return Err(vnode);
        }
        let son_addr = self.fs.inode_alloc.lock().alloc();
        let son_inode = addr2inode(son_addr, self.fs.inode_start);
        let son_vnode = Self::new(son_addr, self.fs.clone(), self.device.clone());
        son_vnode.modify_inode(|inode| inode.init(InodeType::Dir));
        son_vnode.dir_insert(DirEntry::dot(son_inode));
        son_vnode.dir_insert(DirEntry::dotdot(self.inode_idx()));
        self.dir_insert(DirEntry::new(name, son_inode));
        Ok(son_vnode)
    }

    pub fn create(&self, name: &str) -> Result<Arc<Vnode>, Arc<Vnode>> {
        if let Some(vnode) = self.dir_find(name) {
            return Err(vnode);
        }
        let son_addr = self.fs.inode_alloc.lock().alloc();
        let son_inode = addr2inode(son_addr, self.fs.inode_start);
        let son_vnode = Self::new(son_addr, self.fs.clone(), self.device.clone());
        son_vnode.modify_inode(|inode| inode.init(InodeType::File));
        self.dir_insert(DirEntry::new(name, son_inode));
        Ok(son_vnode)
    }

    pub fn read_from(&self, offset: u32, buf: &mut [u8]) -> u32 {
        self.read_inode(|inode| inode.read_from(offset, buf, &self.device))
    }

    pub fn write_from(&self, offset: u32, buf: &[u8]) -> u32 {
        let alloc = self.read_inode(|inode| {
            let need = inode.new_needed_blocks(offset + buf.len() as u32);
            let mut allocator = self.fs.data_alloc.lock();
            (0..need).fold(Vec::new(), |mut acc, _| {
                acc.push(allocator.alloc());
                acc
            })
        });
        self.modify_inode(|inode| inode.write_from_maybe_grow(offset, buf, &self.device, alloc))
    }

    pub fn size(&self) -> u32 {
        self.read_inode(|inode| inode.size)
    }

    pub fn clear(&self) {
        let mut alloc = self.fs.data_alloc.lock();
        self.modify_inode(|inode| inode.clear(&self.device))
            .into_iter()
            .for_each(|addr| alloc.dealloc(addr));
    }

    pub fn is_file(&self) -> bool {
        self.read_inode(|inode| inode.is_file())
    }

    pub fn is_dir(&self) -> bool {
        self.read_inode(|inode| inode.is_dir())
    }
}
