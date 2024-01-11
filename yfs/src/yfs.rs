// use core::mem::size_of;
//
// use spin::mutex::Mutex;
//
// use alloc::sync::Arc;
//
// use crate::{
//     block_alloc::{DataBitmap, InodeAlloc, InodeBitmap},
//     block_cache::BLOCK_CACHE,
//     block_dev::BlockDevice,
//     constant::{BlockAddr, InodeAddr, BLOCK_BITS, BLOCK_SIZE, SUPER},
//     layout::{DirEntry, Inode, InodeType, SuperBlock},
//     vfs::Vnode,
// };
//
// #[derive(Debug)]
// pub struct YeFs {
//     pub device: Arc<dyn BlockDevice>,
//     pub inode_alloc: Mutex<InodeBitmap>,
//     pub data_alloc: Mutex<DataBitmap>,
//     pub inode_start: BlockAddr,
//     pub data_start: BlockAddr,
//     pub root_inode: InodeAddr,
// }
//
// impl YeFs {
//     pub fn format(device: Arc<dyn BlockDevice>, total: u32, inode_bitmap_blocks: u32) -> Arc<Self> {
//         let inode_bitmap = InodeBitmap::new(1, inode_bitmap_blocks, Arc::clone(&device));
//         let inode_max_num = inode_bitmap.size();
//         let inode_area_blocks =
//             (inode_max_num * size_of::<Inode>() as u32 + BLOCK_SIZE as u32 - 1) / BLOCK_SIZE as u32;
//         let inode_total = inode_bitmap_blocks + inode_area_blocks;
//
//         let data_total = total - inode_total - 1;
//         let data_bitmap_blocks = (data_total + BLOCK_BITS as u32) / (BLOCK_BITS as u32 + 1);
//         let data_area_blocks = data_total - data_bitmap_blocks;
//         let data_bitmap = DataBitmap::new(inode_total + 1, data_bitmap_blocks, Arc::clone(&device));
//
//         let fs = Self {
//             device,
//             inode_start: 1 + inode_bitmap_blocks,
//             data_start: 1 + inode_total + data_bitmap_blocks,
//             root_inode: (inode_bitmap_blocks + 1, 0),
//             inode_alloc: Mutex::new(inode_bitmap),
//             data_alloc: Mutex::new(data_bitmap),
//         };
//
//         (0..total).for_each(|addr| {
//             { BLOCK_CACHE.lock().entry(addr, Arc::clone(&fs.device)) }
//                 .lock()
//                 .clear()
//         });
//
//         { BLOCK_CACHE.lock().entry(SUPER, Arc::clone(&fs.device)) }
//             .lock()
//             .modify(|block: &mut SuperBlock| {
//                 block.init(
//                     total,
//                     inode_bitmap_blocks,
//                     inode_area_blocks,
//                     data_bitmap_blocks,
//                     data_area_blocks,
//                     fs.root_inode,
//                 )
//             });
//         assert!(
//             fs.inode_alloc.lock().alloc() == fs.root_inode,
//             "unexpected root inode"
//         );
//         let (addr, _) = fs.root_inode;
//         { BLOCK_CACHE.lock().entry(addr, Arc::clone(&fs.device)) }
//             .lock()
//             .modify(|inode: &mut Inode| {
//                 inode.init(InodeType::Dir);
//             });
//         let fs = Arc::new(fs);
//         let root = Self::root(fs.clone());
//         root.dir_insert(DirEntry::dot(0));
//         fs
//     }
//
//     pub fn load(device: Arc<dyn BlockDevice>) -> Option<Arc<Self>> {
//         { BLOCK_CACHE.lock().entry(SUPER, Arc::clone(&device)) }
//             .lock()
//             .read(|block: &SuperBlock| {
//                 if !block.valid() {
//                     return None;
//                 }
//                 let inode_total = block.inode_bitmap_cnt + block.inode_bitmap_cnt;
//                 let inode_bitmap = InodeBitmap::new(1, block.inode_bitmap_cnt, Arc::clone(&device));
//                 let data_bitmap =
//                     DataBitmap::new(1 + inode_total, block.data_bitmap_cnt, Arc::clone(&device));
//
//                 let fs = Self {
//                     device,
//                     inode_start: 1 + block.inode_bitmap_cnt,
//                     data_start: 1 + inode_total + block.data_bitmap_cnt,
//                     root_inode: block.root_inode,
//                     inode_alloc: Mutex::new(inode_bitmap),
//                     data_alloc: Mutex::new(data_bitmap),
//                 };
//                 Some(Arc::new(fs))
//             })
//     }
//
//     pub fn root(fs: Arc<Self>) -> Arc<Vnode> {
//         let device = Arc::clone(&fs.device);
//         let addr = fs.root_inode;
//         Vnode::new(addr, fs, device)
//     }
//
//     pub fn flush(&self) {
//         BLOCK_CACHE.lock().sync()
//     }
// }
