use core::mem::size_of;
use spin::mutex::Mutex;

use crate::{
    block_alloc::DataBlockAlloc,
    block_cache::BLOCK_CACHE,
    block_dev::BlockDevice,
    constant::{Block, BlockAddr, InodeAddr, BLOCK_SIZE, NULL},
};
use alloc::{sync::Arc, vec::Vec};

const INODE_DIRECT_COUNT: usize = 28;
const INDEX_ENTRY_COUNT: usize = BLOCK_SIZE / size_of::<BlockAddr>();
const INDIRECT1_COUNT: usize = INDEX_ENTRY_COUNT;
const INDIRECT2_COUNT: usize = INDEX_ENTRY_COUNT * INDEX_ENTRY_COUNT;
const INDIRECT1_BOUND: usize = INDIRECT1_COUNT + INODE_DIRECT_COUNT;
const INDIRECT2_BOUND: usize = INDIRECT2_COUNT + INDIRECT1_BOUND;
// 最大支持的文件大小大概是0x813800bytes， 大概是8MB
const MAX_FILE_SIZE: u32 =
    (INODE_DIRECT_COUNT + INDIRECT1_COUNT + INDIRECT2_COUNT) as u32 * BLOCK_SIZE as u32;

pub type INodeBlock = [Inode; BLOCK_SIZE / size_of::<Inode>()];
pub type IndexBlock = [BlockAddr; BLOCK_SIZE / size_of::<BlockAddr>()];
pub type DataBlock = Block;
pub const NAME_LEN_LIMIT: usize = 26;
pub const DIR_ENTRY_COUNT: usize = BLOCK_SIZE / size_of::<DirEntry>();
pub type DirEntryBlock = [DirEntry; DIR_ENTRY_COUNT];

#[repr(C)]
pub struct SuperBlock {
    pub magic: u32,
    pub total_cnt: u32,
    pub inode_bitmap_cnt: u32,
    pub inode_area_cnt: u32,
    pub data_bitmap_cnt: u32,
    pub data_area_cnt: u32,
    pub root_inode: InodeAddr,
}

impl SuperBlock {
    pub const MAGIC: u32 = 0x54321234;
    pub fn bare() -> Self {
        Self {
            magic: 0,
            total_cnt: 0,
            inode_bitmap_cnt: 0,
            inode_area_cnt: 0,
            data_bitmap_cnt: 0,
            data_area_cnt: 0,
            root_inode: (NULL, 0),
        }
    }

    pub fn init(
        &mut self,
        total_cnt: u32,
        inode_bitmap_cnt: u32,
        inode_area_cnt: u32,
        data_bitmap_cnt: u32,
        data_area_cnt: u32,
        root_inode: InodeAddr,
    ) {
        self.magic = Self::MAGIC;
        self.total_cnt = total_cnt;
        self.inode_bitmap_cnt = inode_bitmap_cnt;
        self.inode_area_cnt = inode_area_cnt;
        self.data_bitmap_cnt = data_bitmap_cnt;
        self.data_area_cnt = data_area_cnt;
        self.root_inode = root_inode;
    }

    pub fn valid(&self) -> bool {
        self.magic == Self::MAGIC
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InodeType {
    File,
    Dir,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Inode {
    pub size: u32,
    pub direct: [u32; INODE_DIRECT_COUNT],
    pub indirect1: u32,
    pub indirect2: u32,
    inode_type: InodeType,
}

impl Inode {
    pub fn init(&mut self, inode_type: InodeType) {
        self.size = 0;
        self.inode_type = inode_type;
        self.direct = [NULL; INODE_DIRECT_COUNT];
        self.indirect1 = NULL;
        self.indirect2 = NULL;
    }

    pub fn is_file(&self) -> bool {
        self.inode_type == InodeType::File
    }

    pub fn is_dir(&self) -> bool {
        self.inode_type == InodeType::Dir
    }

    pub fn nth_data_block(&self, n: u32, device: &Arc<dyn BlockDevice>) -> BlockAddr {
        let addr = if n < INODE_DIRECT_COUNT as u32 {
            self.direct[n as usize]
        } else if n < INDIRECT1_BOUND as u32 {
            assert!(self.indirect1 != NULL, "unexpected NULL block");
            { BLOCK_CACHE.lock().entry(self.indirect1, Arc::clone(device)) }
                .lock()
                .read(|block: &IndexBlock| block[n as usize - INODE_DIRECT_COUNT])
        } else if n < INDIRECT2_BOUND as u32 {
            let n = n - INDIRECT1_BOUND as u32;
            let idx0 = n / INDIRECT1_COUNT as u32;
            let idx1 = n % INDIRECT1_COUNT as u32;
            assert!(self.indirect2 != NULL, "unexpected NULL block");
            let indirect1 = {
                BLOCK_CACHE
                    .lock()
                    .entry(self.indirect2 as BlockAddr, Arc::clone(device))
            }
            .lock()
            .read(|block: &IndexBlock| block[idx0 as usize]);
            assert!(indirect1 != NULL, "unexpected NULL block");
            { BLOCK_CACHE.lock().entry(indirect1, Arc::clone(device)) }
                .lock()
                .read(|block: &IndexBlock| block[idx1 as usize])
        } else {
            panic!("invalid block index")
        };
        assert!(addr != NULL, "unexpected NULL block");
        addr
    }

    // 分配一个size大小的文件需要多少数据块
    // size 除以 BLOCK_SIZE 向上取整
    fn needed_data_blocks(size: u32) -> u32 {
        (size + BLOCK_SIZE as u32 - 1) / BLOCK_SIZE as u32
    }

    // 分配一个size大小的文件需要多少块(包括索引快)
    fn needed_total_blocks(size: u32) -> u32 {
        assert!(size <= MAX_FILE_SIZE, "file too large");
        let data_blocks = Self::needed_data_blocks(size);
        let mut total = data_blocks;

        //额外需要一个一级索引块
        if data_blocks > INODE_DIRECT_COUNT as u32 {
            total += 1;
        }

        if data_blocks > INDIRECT1_BOUND as u32 {
            //二级索引块
            total += 1;
            //隶属于二级索引块的一级索引块
            total += (data_blocks - INDIRECT1_BOUND as u32 + INDEX_ENTRY_COUNT as u32 - 1)
                / INDEX_ENTRY_COUNT as u32;
        }
        total
    }

    pub fn data_blocks(&self) -> u32 {
        Self::needed_data_blocks(self.size)
    }

    pub fn total_blocks(&self) -> u32 {
        Self::needed_total_blocks(self.size)
    }

    pub fn new_needed_blocks(&self, new_size: u32) -> u32 {
        let new = Self::needed_total_blocks(new_size);
        let old = Self::needed_total_blocks(self.size);
        if new > old {
            new - old
        } else {
            0
        }
    }

    //为inode分配新的数据块来适应新的文件大小
    pub fn grow(&mut self, new_size: u32, alloc: Vec<BlockAddr>, device: &Arc<dyn BlockDevice>) {
        assert!(
            new_size >= self.size,
            "new size must be larger than old size"
        );
        let mut alloc = alloc.into_iter();
        let mut current_data_blocks = self.data_blocks();
        let new_data_blocks = Self::needed_data_blocks(new_size);
        self.size = new_size;
        //分配直接索引块
        while current_data_blocks < new_data_blocks.min(INODE_DIRECT_COUNT as u32) {
            self.direct[current_data_blocks as usize] = alloc.next().unwrap();
            current_data_blocks += 1;
        }

        //分配一级间接索引块
        if new_data_blocks <= INODE_DIRECT_COUNT as u32 {
            return;
        } else if current_data_blocks == INODE_DIRECT_COUNT as u32 {
            self.indirect1 = alloc.next().unwrap();
        }

        //间接索引块管辖的数据块
        let mut current_indirect_data_blocks = current_data_blocks - INODE_DIRECT_COUNT as u32;
        let new_indirect_data_blocks = new_data_blocks - INODE_DIRECT_COUNT as u32;
        //分配一级间接索引块管辖的数据块
        { BLOCK_CACHE.lock().entry(self.indirect1, Arc::clone(device)) }
            .lock()
            .modify(|indirect1: &mut IndexBlock| {
                while current_indirect_data_blocks
                    < new_indirect_data_blocks.min(INDIRECT1_COUNT as u32)
                {
                    indirect1[current_indirect_data_blocks as usize] = alloc.next().unwrap();
                    current_indirect_data_blocks += 1;
                }
            });

        //分配二级间接索引块
        if new_indirect_data_blocks <= INDIRECT1_COUNT as u32 {
            return;
        } else if current_indirect_data_blocks == INDIRECT1_COUNT as u32 {
            self.indirect2 = alloc.next().unwrap();
        }

        //二级间接索引块管辖的数据块
        let current_indirect2_data_blocks = current_indirect_data_blocks - INDIRECT1_COUNT as u32;
        let new_indirect2_data_blocks = new_indirect_data_blocks - INDIRECT1_COUNT as u32;
        let mut current_indirect2_data_blocks_idx0 =
            current_indirect2_data_blocks / INDIRECT1_COUNT as u32;
        let mut current_indirect2_data_blocks_idx1 =
            current_indirect2_data_blocks % INDIRECT1_COUNT as u32;
        let new_indirect2_data_blocks_idx0 = new_indirect2_data_blocks / INDIRECT1_COUNT as u32;
        let new_indirect2_data_blocks_idx1 = new_indirect2_data_blocks % INDIRECT1_COUNT as u32;

        { BLOCK_CACHE.lock().entry(self.indirect2, Arc::clone(device)) }
            .lock()
            .modify(|indirect2: &mut IndexBlock| {
                while current_indirect2_data_blocks_idx0 < new_indirect2_data_blocks_idx0
                    || current_indirect2_data_blocks_idx0 == new_indirect2_data_blocks_idx0
                        && current_indirect2_data_blocks_idx1 < new_indirect2_data_blocks_idx1
                {
                    if current_indirect2_data_blocks_idx1 == 0 {
                        //现在current指向了新的一级索引块, 所以要分配一级索引块
                        indirect2[current_indirect2_data_blocks_idx0 as usize] =
                            alloc.next().unwrap();
                    }
                    //读取一级索引块
                    {
                        BLOCK_CACHE.lock().entry(
                            indirect2[current_indirect2_data_blocks_idx0 as usize],
                            Arc::clone(device),
                        )
                    }
                    .lock()
                    .modify(|indirect1: &mut IndexBlock| {
                        indirect1[current_indirect2_data_blocks_idx1 as usize] =
                            alloc.next().unwrap();
                    });
                    current_indirect2_data_blocks_idx1 += 1;
                    if current_indirect2_data_blocks_idx1 == INDIRECT1_COUNT as u32 {
                        current_indirect2_data_blocks_idx1 = 0;
                        current_indirect2_data_blocks_idx0 += 1;
                    }
                }
            });
    }

    pub fn trunc(&mut self, new_size: u32, device: &Arc<dyn BlockDevice>) -> Vec<BlockAddr> {
        assert!(
            new_size <= self.size,
            "new size must be smaller than old size"
        );
        let mut ret = vec![];
        let current_data_blocks = self.data_blocks();
        let new_data_blocks = Self::needed_data_blocks(new_size);
        self.size = new_size;

        // 回收二级间接索引块管辖的数据块
        if current_data_blocks > INDIRECT1_BOUND as u32 {
            let from = current_data_blocks - INDIRECT1_BOUND as u32;
            // 如果new_data_blocks小于INDIRECT1_BOUND, 则回收所有二级间接索引块管辖的数据块
            // 如果new_data_blocks大于INDIRECT1_BOUND, 则回收部分二级间接索引块管辖的数据块
            let to = new_data_blocks.max(INDIRECT1_BOUND as u32) - INDIRECT1_BOUND as u32;
            let mut idx0 = from / INDEX_ENTRY_COUNT as u32;
            let mut idx1 = from % INDEX_ENTRY_COUNT as u32;
            let to_idx0 = to / INDEX_ENTRY_COUNT as u32;
            let to_idx1 = to % INDEX_ENTRY_COUNT as u32;
            { BLOCK_CACHE.lock().entry(self.indirect2, Arc::clone(device)) }
                .lock()
                .modify(|indirect2: &mut IndexBlock| {
                    while idx0 > to_idx0 && idx0 == to_idx0 || idx1 > to_idx1 {
                        if idx1 == 0u32 {
                            idx1 = INDEX_ENTRY_COUNT as u32 - 1;
                            idx0 -= 1;
                        } else {
                            idx1 -= 1;
                        }
                        {
                            BLOCK_CACHE
                                .lock()
                                .entry(indirect2[idx0 as usize], Arc::clone(device))
                        }
                        .lock()
                        .modify(|indirect1: &mut IndexBlock| {
                            ret.push(indirect1[idx1 as usize]);
                            indirect1[idx1 as usize] = NULL;
                        });
                        if idx1 == 0 {
                            ret.push(indirect2[idx0 as usize]);
                            indirect2[idx0 as usize] = NULL;
                        }
                    }
                });
            if (to_idx0, to_idx1) == (0u32, 0u32) {
                ret.push(self.indirect2);
                self.indirect2 = NULL;
            }
        }

        //回收一级间接索引块管辖的数据块
        if current_data_blocks > INODE_DIRECT_COUNT as u32 {
            let mut idx =
                (current_data_blocks - INODE_DIRECT_COUNT as u32).min(INDEX_ENTRY_COUNT as u32);
            let to = new_data_blocks.max(INODE_DIRECT_COUNT as u32) - INODE_DIRECT_COUNT as u32;
            {
                BLOCK_CACHE
                    .lock()
                    .entry(self.indirect1, Arc::clone(&device))
            }
            .lock()
            .modify(|indirect1: &mut IndexBlock| {
                while idx > to {
                    idx -= 1;
                    ret.push(indirect1[idx as usize]);
                    indirect1[idx as usize] = NULL;
                }
            });
            if to == 0u32 {
                ret.push(self.indirect1);
                self.indirect1 = NULL;
            }
        }

        let mut idx = current_data_blocks.min(INODE_DIRECT_COUNT as u32);
        let to = new_data_blocks.min(INODE_DIRECT_COUNT as u32);
        while idx > to {
            idx -= 1;
            ret.push(self.direct[idx as usize]);
            self.direct[idx as usize] = NULL;
        }
        ret
    }

    pub fn clear(&mut self, device: &Arc<dyn BlockDevice>) -> Vec<BlockAddr> {
        self.trunc(0u32, device)
    }

    // //释放所有数据块并将inode的size置为0
    // pub fn _clear(&mut self, device: &Arc<dyn BlockDevice>, allocator: &mut impl DataBlockAlloc) {
    //     let total = self.data_blocks();
    //     self.size = 0;
    //     let mut current = 0;
    //     //释放直接索引的数据块
    //     while current < total.min(INODE_DIRECT_COUNT as u32) {
    //         allocator.dealloc(self.direct[current as usize]);
    //         self.direct[current as usize] = NULL;
    //         current += 1;
    //     }
    //     if total <= INODE_DIRECT_COUNT as u32 {
    //         return;
    //     }
    //
    //     //释放一级间接索引块和其管辖的数据块
    //     let indirect_blocks_num = total - INODE_DIRECT_COUNT as u32;
    //     let mut current = 0;
    //     { BLOCK_CACHE.lock().entry(self.indirect1, Arc::clone(device)) }
    //         .lock()
    //         .modify(|indirect1: &mut IndexBlock| {
    //             while current < indirect_blocks_num.min(INDIRECT1_COUNT as u32) {
    //                 allocator.dealloc(indirect1[current as usize]);
    //                 // indirect1[current as usize] = NULL;
    //                 current += 1;
    //             }
    //         });
    //     allocator.dealloc(self.indirect1);
    //     self.indirect1 = NULL;
    //     if total <= INDIRECT1_BOUND as u32 {
    //         return;
    //     }
    //     //释放二级间接索引块和其管辖的数据块
    //     let indirect2_blocks_num = indirect_blocks_num - INDIRECT1_COUNT as u32;
    //     let idx0 = indirect2_blocks_num / INDEX_ENTRY_COUNT as u32;
    //     let idx1 = indirect2_blocks_num % INDEX_ENTRY_COUNT as u32;
    //     { BLOCK_CACHE.lock().entry(self.indirect2, Arc::clone(device)) }
    //         .lock()
    //         .modify(|indirect2: &mut IndexBlock| {
    //             for &mut entry in indirect2.iter_mut().take(idx0 as usize) {
    //                 { BLOCK_CACHE.lock().entry(entry, Arc::clone(device)) }
    //                     .lock()
    //                     .modify(|indirect1: &mut IndexBlock| {
    //                         indirect1
    //                             .iter_mut()
    //                             .for_each(|&mut entry| allocator.dealloc(entry))
    //                     });
    //                 allocator.dealloc(entry);
    //             }
    //
    //             if idx1 > 0 {
    //                 {
    //                     BLOCK_CACHE
    //                         .lock()
    //                         .entry(indirect2[idx0 as usize], Arc::clone(device))
    //                 }
    //                 .lock()
    //                 .modify(|indirect1: &mut IndexBlock| {
    //                     for &mut entry in indirect1.iter_mut().take(idx1 as usize) {
    //                         allocator.dealloc(entry);
    //                     }
    //                 });
    //                 allocator.dealloc(indirect2[idx0 as usize]);
    //             }
    //         });
    //     allocator.dealloc(self.indirect2);
    //     self.indirect2 = NULL;
    // }

    pub fn read_from(&self, offset: u32, buf: &mut [u8], device: &Arc<dyn BlockDevice>) -> u32 {
        let mut iter = unsafe {
            FileDataIter::unlocate(
                &mut *(self as *const _ as usize as *mut _),
                Arc::clone(device),
            )
        };
        iter.seek(SeekFrom::Start(offset));
        iter.read(buf)
    }

    pub fn write_from(&mut self, offset: u32, buf: &[u8], device: &Arc<dyn BlockDevice>) -> u32 {
        let mut iter = unsafe { FileDataIter::unlocate(self, Arc::clone(device)) };
        iter.seek(SeekFrom::Start(offset));
        iter.write(buf)
    }

    pub fn write_from_maybe_grow(
        &mut self,
        offset: u32,
        buf: &[u8],
        device: &Arc<dyn BlockDevice>,
        alloc: Vec<BlockAddr>,
    ) -> u32 {
        if self.size < offset + buf.len() as u32 {
            self.grow(offset + buf.len() as u32, alloc, device)
        }
        let mut iter = unsafe { FileDataIter::unlocate(self, Arc::clone(device)) };
        iter.seek(SeekFrom::Start(offset));
        iter.write(buf)
    }

    pub fn append(&mut self, buf: &[u8], device: &Arc<dyn BlockDevice>, alloc: Vec<BlockAddr>) {
        let old_size = self.size;
        let new_size = old_size + buf.len() as u32;
        self.grow(new_size, alloc, device);
        let mut iter = unsafe { FileDataIter::unlocate(self, Arc::clone(device)) };
        iter.seek(SeekFrom::Start(old_size));
        iter.write(buf);
    }

    pub fn dir_insert(
        &mut self,
        to_insert: DirEntry,
        device: &Arc<dyn BlockDevice>,
        alloc: Vec<BlockAddr>,
    ) {
        assert!(self.is_dir(), "only dir can insert");
        self.append(to_insert.as_bytes(), device, alloc);
    }

    pub fn dir(&mut self, device: &Arc<dyn BlockDevice>) -> Vec<DirEntry> {
        assert!(self.is_dir(), "only dir can list");
        Dir::new(self, Arc::clone(device))
            .filter(|entry| entry.valid)
            .collect()
    }

    pub fn dir_delete(
        &mut self,
        name: &str,
        device: &Arc<dyn BlockDevice>,
    ) -> Result<DirEntry, ()> {
        assert!(self.is_dir(), "only dir can delete");
        let mut dir = Dir::new(self, Arc::clone(device));
        dir.delete(name)
    }

    pub fn dir_find(&self, name: &str, device: &Arc<dyn BlockDevice>) -> Option<DirEntry> {
        assert!(self.is_dir(), "only dir can find");
        let mut dir = Dir::new(
            &mut unsafe { *(self as *const Inode as usize as *mut Inode) },
            Arc::clone(device),
        );
        dir.find(|entry| entry.name() == name && entry.valid)
    }
}

#[allow(unused)]
enum SeekFrom {
    Start(u32),
    Cur(i32),
}

#[derive(Clone)]
pub struct FileDataIter {
    inode: *mut Inode,
    device: Arc<dyn BlockDevice>,
    // 当前数据块的索引号
    block_idx: u32,
    // 当前读写位置的块内偏移
    block_offset: u32,
    // 当前数据块的地址
    block_addr: BlockAddr,
}

impl FileDataIter {
    fn inode(&self) -> &mut Inode {
        unsafe { &mut *self.inode }
    }

    fn file_size(&mut self) -> u32 {
        self.inode().size
    }

    //当前读写位置相对于文件字节偏移
    fn offset(&self) -> u32 {
        self.block_idx * BLOCK_SIZE as u32 + self.block_offset
    }

    fn new(inode: &mut Inode, device: Arc<dyn BlockDevice>) -> Self {
        Self {
            inode: inode as *mut _,
            block_idx: 0,
            block_offset: 0,
            block_addr: inode.nth_data_block(0, &device),
            device,
        }
    }

    //还没有定位到文件首个数据块, 使用的时候需要先调用一次seek
    unsafe fn unlocate(inode: &mut Inode, device: Arc<dyn BlockDevice>) -> Self {
        Self {
            inode: inode as *mut _,
            block_idx: 0,
            block_offset: 0,
            block_addr: NULL,
            device,
        }
    }

    fn seek(&mut self, seek: SeekFrom) {
        let to = match seek {
            SeekFrom::Start(to) => to,
            SeekFrom::Cur(step) => self.offset() + step as u32,
        };
        assert!(to < self.file_size(), "seek out of range");
        self.block_idx = to / BLOCK_SIZE as u32;
        self.block_offset = to % BLOCK_SIZE as u32;
        self.block_addr = self.inode().nth_data_block(self.block_idx, &self.device);
    }

    fn step(&mut self, step: u32) {
        let to = self.offset() + step;
        assert!(to < self.file_size(), "step out of range");
        self.block_idx = to / BLOCK_SIZE as u32;
        self.block_offset = to % BLOCK_SIZE as u32;
        self.block_addr = self.inode().nth_data_block(self.block_idx, &self.device);
    }

    fn read(&mut self, buf: &mut [u8]) -> u32 {
        // 本次读的结尾相对于文件首的字节偏移
        let end = (self.offset() + buf.len() as u32).min(self.file_size());
        if end >= self.offset() {
            return 0;
        }
        let mut read = 0u32;
        loop {
            //本次拷贝位置相对于文件头的字节偏移
            let this_cpy_end = ((self.block_idx + 1) * BLOCK_SIZE as u32).min(end);
            let this_cpy_size = this_cpy_end - self.offset();
            let dest = &mut buf[read as usize..(read + this_cpy_size) as usize];
            {
                BLOCK_CACHE
                    .lock()
                    .entry(self.block_addr, Arc::clone(&self.device))
            }
            .lock()
            .read(|block: &Block| {
                let src = &block
                    [self.block_offset as usize..(self.block_offset + this_cpy_size) as usize];
                dest.copy_from_slice(src);
            });
            read += this_cpy_size;
            if this_cpy_end == end {
                break;
            }
            self.step(this_cpy_size);
        }
        read
    }

    /// 必须保证文件大小在调用前调整到能够容纳数据的大小
    fn write(&mut self, buf: &[u8]) -> u32 {
        // 本次读的结尾相对于文件首的字节偏移
        let end = (self.offset() + buf.len() as u32).min(self.file_size());
        if end <= self.offset() {
            return 0;
        }
        let mut write = 0u32;
        loop {
            //本次拷贝位置相对于文件头的字节偏移
            let this_cpy_end = ((self.block_idx + 1) * BLOCK_SIZE as u32).min(end);
            let this_cpy_size = this_cpy_end - self.offset();
            let src = &buf[write as usize..(write + this_cpy_size) as usize];
            {
                BLOCK_CACHE
                    .lock()
                    .entry(self.block_addr, Arc::clone(&self.device))
            }
            .lock()
            .modify(|block: &mut Block| {
                let dest = &mut block
                    [self.block_offset as usize..(self.block_offset + this_cpy_size) as usize];
                dest.copy_from_slice(src);
            });
            write += this_cpy_size;
            if this_cpy_end == end {
                break;
            }
            self.step(this_cpy_size);
        }
        write
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DirEntry {
    pub valid: bool,
    pub name: [u8; NAME_LEN_LIMIT + 1],
    pub inode_idx: u32,
}

impl DirEntry {
    pub fn bare() -> Self {
        Default::default()
    }

    pub fn new(name: &str, inode_idx: u32) -> Self {
        assert!(name.len() <= NAME_LEN_LIMIT);
        let mut bytes = [0u8; NAME_LEN_LIMIT + 1];
        bytes[..name.len()].copy_from_slice(name.as_bytes());
        Self {
            name: bytes,
            inode_idx,
            valid: true,
        }
    }

    pub fn dot(inode_idx: u32) -> Self {
        Self {
            name: [
                b'.', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            inode_idx,
            valid: true,
        }
    }

    pub fn dotdot(inode_idx: u32) -> Self {
        Self {
            name: [
                b'.', b'.', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0,
            ],
            inode_idx,
            valid: true,
        }
    }

    pub fn name(&self) -> &str {
        let len = self
            .name
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(NAME_LEN_LIMIT);
        core::str::from_utf8(&self.name[..len]).unwrap()
    }

    pub fn inode_idx(&self) -> u32 {
        self.inode_idx
    }

    pub fn as_bytes(&self) -> &[u8; size_of::<Self>()] {
        assert!(size_of::<Self>() == 32);
        unsafe { &*(self as *const _ as *const [u8; size_of::<Self>()]) }
    }

    pub fn as_bytes_mut(&self) -> &mut [u8; size_of::<Self>()] {
        unsafe { &mut *(self as *const _ as *mut [u8; size_of::<Self>()]) }
    }
}

struct Dir {
    inode: *mut Inode,
    block_idx: u32,
    block_offset: u32,
    block: DirEntryBlock,
    device: Arc<dyn BlockDevice>,
}

impl Iterator for Dir {
    type Item = DirEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.block_offset * size_of::<DirEntry>() as u32 + self.block_idx * BLOCK_SIZE as u32
            >= self.inode().size
        {
            return None;
        }
        if self.block_offset == 0 {
            {
                BLOCK_CACHE.lock().entry(
                    self.inode().nth_data_block(self.block_idx, &self.device),
                    Arc::clone(&self.device),
                )
            }
            .lock()
            .read(|block: &DirEntryBlock| self.block = *block);
        }
        let ret = self.block[self.block_offset as usize];
        self.block_offset += 1;
        if self.block_offset == DIR_ENTRY_COUNT as u32 {
            self.block_offset = 0;
            self.block_idx += 1;
        }
        Some(ret)
    }
}

impl Dir {
    fn new(inode: &mut Inode, device: Arc<dyn BlockDevice>) -> Self {
        Self {
            inode,
            block_idx: 0,
            block_offset: 0,
            block: [Default::default(); DIR_ENTRY_COUNT],
            device,
        }
    }

    fn inode(&self) -> &mut Inode {
        unsafe { &mut *self.inode }
    }

    fn delete(&mut self, to_delete: &str) -> Result<DirEntry, ()> {
        while (self.block_offset * size_of::<DirEntry>() as u32
            + self.block_idx * BLOCK_SIZE as u32)
            < self.inode().size
        {
            if self.block_offset == 0 {
                {
                    BLOCK_CACHE.lock().entry(
                        self.inode().nth_data_block(self.block_idx, &self.device),
                        Arc::clone(&self.device),
                    )
                }
                .lock()
                .read(|block: &DirEntryBlock| self.block = *block);
            }
            let entry = self.block[self.block_offset as usize];
            if entry.valid && entry.name() == to_delete {
                {
                    BLOCK_CACHE.lock().entry(
                        self.inode().nth_data_block(self.block_idx, &self.device),
                        Arc::clone(&self.device),
                    )
                }
                .lock()
                .modify(|block: &mut DirEntryBlock| {
                    block[self.block_offset as usize].valid = false;
                });
                return Ok(entry);
            }
            self.block_offset += 1;
            if self.block_offset == DIR_ENTRY_COUNT as u32 {
                self.block_offset = 0;
                self.block_idx += 1;
            }
        }
        return Err(());
    }
}
