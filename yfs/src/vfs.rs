use alloc::{sync::Arc, vec::Vec};
use core::mem::size_of;

use crate::{
    block_cache::cache_entry,
    block_dev::BlockDevice,
    constant::{addr2inode, inode2addr, InodeAddr},
    layout::{DirEntry, Directory, Inode, InodeType},
    yfs::YeFs,
};

/// 对磁盘上的inode对象的引用
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
        let v = cache_entry(self.addr.0, Arc::clone(&self.device))
            .lock()
            .read_at(self.addr.1 * size_of::<Inode>() as u32, f);
        v
    }

    pub fn modify_inode<V>(&self, f: impl FnOnce(&mut Inode) -> V) -> V {
        let v = cache_entry(self.addr.0, Arc::clone(&self.device))
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

    pub unsafe fn dir_insert(&self, entry: DirEntry) {
        self.modify_inode(|inode| inode.dir_insert(entry, &self.device, &self.fs.data_allocator))
    }

    pub fn dir_rm(&self, name: &str) -> Result<(), ()> {
        let res = self.modify_inode(|inode| inode.dir_delete(name, &self.device));
        match res {
            Some(entry) => {
                let addr = inode2addr(entry.inode_idx, self.fs.inode_start);
                let to_delete = Self::new(addr, self.fs.clone(), self.device.clone());
                to_delete.modify_inode(|inode| inode.clear(&self.fs.data_allocator, &self.device));
                self.fs.inode_allocator.dealloc(addr);
                Ok(())
            }
            None => Err(()),
        }
    }

    pub fn ls(&self) -> Vec<DirEntry> {
        self.read_inode(|inode| inode.dir_entries(&self.device))
    }

    pub fn mkdir(&self, name: &str) -> Result<Arc<Vnode>, Arc<Vnode>> {
        if let Some(vnode) = self.dir_find(name) {
            return Err(vnode);
        }
        let son_addr = self.fs.inode_allocator.alloc();
        let son_inode = addr2inode(son_addr, self.fs.inode_start);
        let son_vnode = Self::new(son_addr, self.fs.clone(), self.device.clone());
        son_vnode.modify_inode(|inode| inode.init(InodeType::Dir));
        unsafe { son_vnode.dir_insert(DirEntry::dot(son_inode)) };
        unsafe { son_vnode.dir_insert(DirEntry::dotdot(self.inode_idx())) };
        unsafe { self.dir_insert(DirEntry::new(name, son_inode)) };
        Ok(son_vnode)
    }

    pub fn create(&self, name: &str) -> Result<Arc<Vnode>, Arc<Vnode>> {
        if let Some(vnode) = self.dir_find(name) {
            return Err(vnode);
        }
        let son_addr = self.fs.inode_allocator.alloc();
        let son_inode = addr2inode(son_addr, self.fs.inode_start);
        let son_vnode = Self::new(son_addr, self.fs.clone(), self.device.clone());
        son_vnode.modify_inode(|inode| inode.init(InodeType::File));
        unsafe { self.dir_insert(DirEntry::new(name, son_inode)) };
        Ok(son_vnode)
    }

    pub fn read(&self, offset: u32, buf: &mut [u8]) -> u32 {
        self.read_inode(|inode| inode.read(offset, buf, &self.device))
    }

    pub fn write(&self, offset: u32, buf: &[u8]) -> u32 {
        self.modify_inode(|inode| {
            inode.write_may_grow(offset, buf, &self.device, &self.fs.data_allocator)
        })
    }

    pub fn size(&self) -> u32 {
        self.read_inode(|inode| inode.size)
    }

    pub fn clear(&self) {
        self.modify_inode(|inode| inode.clear(&self.fs.data_allocator, &self.device))
    }

    pub fn is(&self, ty: InodeType) -> bool {
        self.read_inode(|inode| inode.is(ty))
    }

    pub fn is_file(&self) -> bool {
        self.is(InodeType::File)
    }

    pub fn is_dir(&self) -> bool {
        self.is(InodeType::Dir)
    }
}
