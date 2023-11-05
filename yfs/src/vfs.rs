use alloc::{sync::Arc, vec::Vec};
use core::{mem::size_of, ops::DerefMut};

use crate::{
    block_alloc::InodeAlloc,
    block_cache::BLOCK_CACHE,
    block_dev::BlockDevice,
    constant::{addr2inode, inode2addr, InodeAddr},
    layout::{DirEntry, Inode, InodeType},
    yfs::YeFs,
};

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
        {
            BLOCK_CACHE
                .lock()
                .entry(self.addr.0, Arc::clone(&self.device))
        }
        .lock()
        .read_at(self.addr.1 * size_of::<Inode>() as u32, f)
    }

    pub fn modify_inode<V>(&self, f: impl FnOnce(&mut Inode) -> V) -> V {
        {
            BLOCK_CACHE
                .lock()
                .entry(self.addr.0, Arc::clone(&self.device))
        }
        .lock()
        .modify_at(self.addr.1 * size_of::<Inode>() as u32, f)
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
        self.modify_inode(|inode| {
            inode.dir_insert(entry, &self.device, self.fs.data_alloc.lock().deref_mut())
        })
    }

    pub fn dir_rm(&self, name: &str) -> Result<(), ()> {
        self.modify_inode(|inode| match inode.dir_delete(name, &self.device) {
            Ok(entry) => {
                let inode = entry.inode_idx;
                let addr = inode2addr(inode, self.fs.inode_start);
                let to_delete = Self::new(addr, self.fs.clone(), self.device.clone());
                to_delete.modify_inode(|inode| {
                    inode.clear(
                        &to_delete.device,
                        to_delete.fs.data_alloc.lock().deref_mut(),
                    )
                });
                self.fs.inode_alloc.lock().dealloc(addr);
                return Ok(());
            }
            Err(_) => Err(()),
        })
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
        self.modify_inode(|inode| {
            inode.write_from_maybe_grow(
                offset,
                buf,
                &self.device,
                self.fs.data_alloc.lock().deref_mut(),
            )
        })
    }
}
