use alloc::sync::Arc;
use core::mem::size_of;

use crate::{
    block_alloc::{DataBlockAllocator, InodeAllocator},
    block_cache::{cache_entry, flush},
    block_dev::BlockDevice,
    constant::{BlockAddr, InodeAddr, BLOCK_BITS, BLOCK_SIZE, SUPER},
    layout::{DirEntry, Inode, InodeType, SuperBlock},
    vfs::Vnode,
};

#[derive(Debug)]
pub struct YeFs {
    pub device: Arc<dyn BlockDevice>,
    pub inode_allocator: InodeAllocator,
    pub data_allocator: DataBlockAllocator,
    pub inode_start: BlockAddr,
    pub data_start: BlockAddr,
    pub root_inode: InodeAddr,
}

impl YeFs {
    pub fn format(device: Arc<dyn BlockDevice>, total: u32, inode_bitmap_blocks: u32) -> Arc<Self> {
        let inode_allocator = InodeAllocator::new(1, inode_bitmap_blocks, device.clone());
        let inode_max_num = inode_allocator.size();
        let inode_area_blocks =
            (inode_max_num * size_of::<Inode>() as u32 + BLOCK_SIZE as u32 - 1) / BLOCK_SIZE as u32;
        let inode_total = inode_bitmap_blocks + inode_area_blocks;

        let data_total = total - inode_total - 1;
        let data_bitmap_blocks = (data_total + BLOCK_BITS as u32) / (BLOCK_BITS as u32 + 1);
        let data_area_blocks = data_total - data_bitmap_blocks;
        let data_allocator =
            DataBlockAllocator::new(inode_total + 1, data_bitmap_blocks, device.clone());

        let fs = Self {
            device: device.clone(),
            inode_start: 1 + inode_bitmap_blocks,
            data_start: 1 + inode_total + data_bitmap_blocks,
            root_inode: (inode_bitmap_blocks + 1, 0),
            inode_allocator,
            data_allocator,
        };

        (0..total).for_each(|addr| cache_entry(addr, device.clone()).lock().clear());

        cache_entry(SUPER, device.clone())
            .lock()
            .modify(|block: &mut SuperBlock| {
                block.init(
                    total,
                    inode_bitmap_blocks,
                    inode_area_blocks,
                    data_bitmap_blocks,
                    data_area_blocks,
                    fs.root_inode,
                )
            });
        assert!(
            fs.inode_allocator.alloc() == fs.root_inode,
            "unexpected root inode"
        );
        let (addr, _) = fs.root_inode;
        cache_entry(addr, device.clone())
            .lock()
            .modify(|inode: &mut Inode| {
                inode.init(InodeType::Dir);
            });
        let fs = Arc::new(fs);
        let root = Self::root(fs.clone());
        unsafe { root.dir_insert(DirEntry::dot(0)) };
        fs
    }

    pub fn load(device: Arc<dyn BlockDevice>) -> Option<Arc<Self>> {
        cache_entry(SUPER, Arc::clone(&device))
            .lock()
            .read(|block: &SuperBlock| {
                if !block.valid() {
                    return None;
                }
                let inode_total = block.inode_bitmap_cnt + block.inode_bitmap_cnt;
                let inode_allocator =
                    InodeAllocator::new(1, block.inode_bitmap_cnt, Arc::clone(&device));
                let data_allocator = DataBlockAllocator::new(
                    1 + inode_total,
                    block.data_bitmap_cnt,
                    Arc::clone(&device),
                );

                let fs = Self {
                    device,
                    inode_start: 1 + block.inode_bitmap_cnt,
                    data_start: 1 + inode_total + block.data_bitmap_cnt,
                    root_inode: block.root_inode,
                    inode_allocator,
                    data_allocator,
                };
                Some(Arc::new(fs))
            })
    }

    pub fn root(fs: Arc<Self>) -> Arc<Vnode> {
        let root_inode = fs.root_inode;
        let device = fs.device.clone();
        Vnode::new(root_inode, fs, device)
    }

    pub fn flush(&self) {
        flush()
    }
}
