use core::{fmt::Display, mem::size_of};

use crate::{
    block_alloc::DataBlockAllocator,
    block_cache::cache_entry,
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
pub const NAME_LEN_LIMIT: usize = 26;
pub const DIR_ENTRY_COUNT: usize = BLOCK_SIZE / size_of::<DirEntry>();

pub type INodeBlock = [Inode; BLOCK_SIZE / size_of::<Inode>()];
pub type IndexBlock = [BlockAddr; BLOCK_SIZE / size_of::<BlockAddr>()];
pub type DataBlock = Block;
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

impl Display for InodeType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            InodeType::File => write!(f, "file"),
            InodeType::Dir => write!(f, "dir"),
        }
    }
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

    pub fn is(&self, ty: InodeType) -> bool {
        self.inode_type == ty
    }

    fn assert(&self, ty: InodeType) {
        assert!(self.is(ty), "expect {ty}");
    }

    fn assert_dir(&self) {
        self.assert(InodeType::Dir)
    }

    fn nth_data_block(&self, n: usize, device: &Arc<dyn BlockDevice>) -> BlockAddr {
        let addr = if n < INODE_DIRECT_COUNT {
            self.direct[n]
        } else if n < INDIRECT1_BOUND {
            assert!(self.indirect1 != NULL, "unexpected NULL block");
            cache_entry(self.indirect1, device.clone())
                .lock()
                .read(|block: &IndexBlock| block[n as usize - INODE_DIRECT_COUNT])
        } else if n < INDIRECT2_BOUND {
            let n = n - INDIRECT1_BOUND;
            let idx0 = n / INDIRECT1_COUNT;
            let idx1 = n % INDIRECT1_COUNT;
            assert!(self.indirect2 != NULL, "unexpected NULL block");
            let indirect1 = cache_entry(self.indirect2 as BlockAddr, Arc::clone(device))
                .lock()
                .read(|block: &IndexBlock| block[idx0 as usize]);
            assert!(indirect1 != NULL, "unexpected NULL block");
            cache_entry(indirect1, device.clone())
                .lock()
                .read(|block: &IndexBlock| block[idx1 as usize])
        } else {
            panic!("invalid block index")
        };
        assert!(addr != NULL, "unexpected NULL block");
        addr
    }

    /// 分配一个size大小的文件需要多少数据块
    /// size 除以 BLOCK_SIZE 向上取整
    fn needed_data(size: u32) -> u32 {
        (size + BLOCK_SIZE as u32 - 1) / BLOCK_SIZE as u32
    }

    /// 分配一个size大小的文件需要多少块(包括索引快)
    fn needed_total(size: u32) -> u32 {
        assert!(size <= MAX_FILE_SIZE, "file too large");
        let data_blocks = Self::needed_data(size);
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
        Self::needed_data(self.size)
    }

    pub fn total_blocks(&self) -> u32 {
        Self::needed_total(self.size)
    }

    pub fn extra_needed(&self, new_size: u32) -> u32 {
        let new = Self::needed_total(new_size);
        let old = Self::needed_total(self.size);
        if new > old {
            new - old
        } else {
            0
        }
    }

    //为inode分配新的数据块来适应新的文件大小
    pub fn grow(
        &mut self,
        new_size: u32,
        allocator: &DataBlockAllocator,
        device: &Arc<dyn BlockDevice>,
    ) {
        assert!(
            new_size >= self.size,
            "new size must be larger than old size"
        );
        let mut current_data_blocks = self.data_blocks();
        let new_data_blocks = Self::needed_data(new_size);
        self.size = new_size;
        //分配直接索引块
        while current_data_blocks < new_data_blocks.min(INODE_DIRECT_COUNT as u32) {
            self.direct[current_data_blocks as usize] = allocator.alloc();
            current_data_blocks += 1;
        }

        //分配一级间接索引块
        if new_data_blocks <= INODE_DIRECT_COUNT as u32 {
            return;
        } else if current_data_blocks == INODE_DIRECT_COUNT as u32 {
            self.indirect1 = allocator.alloc();
        }

        //间接索引块管辖的数据块
        let mut current_indirect_data_blocks = current_data_blocks - INODE_DIRECT_COUNT as u32;
        let new_indirect_data_blocks = new_data_blocks - INODE_DIRECT_COUNT as u32;
        //分配一级间接索引块管辖的数据块
        cache_entry(self.indirect1, Arc::clone(device))
            .lock()
            .modify(|indirect1: &mut IndexBlock| {
                while current_indirect_data_blocks
                    < new_indirect_data_blocks.min(INDIRECT1_COUNT as u32)
                {
                    indirect1[current_indirect_data_blocks as usize] = allocator.alloc();
                    current_indirect_data_blocks += 1;
                }
            });

        //分配二级间接索引块
        if new_indirect_data_blocks <= INDIRECT1_COUNT as u32 {
            return;
        } else if current_indirect_data_blocks == INDIRECT1_COUNT as u32 {
            self.indirect2 = allocator.alloc();
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

        cache_entry(self.indirect2, Arc::clone(device))
            .lock()
            .modify(|indirect2: &mut IndexBlock| {
                while current_indirect2_data_blocks_idx0 < new_indirect2_data_blocks_idx0
                    || current_indirect2_data_blocks_idx0 == new_indirect2_data_blocks_idx0
                        && current_indirect2_data_blocks_idx1 < new_indirect2_data_blocks_idx1
                {
                    if current_indirect2_data_blocks_idx1 == 0 {
                        //现在current指向了新的一级索引块, 所以要分配一级索引块
                        indirect2[current_indirect2_data_blocks_idx0 as usize] = allocator.alloc();
                    }
                    //读取一级索引块
                    cache_entry(
                        indirect2[current_indirect2_data_blocks_idx0 as usize],
                        Arc::clone(device),
                    )
                    .lock()
                    .modify(|indirect1: &mut IndexBlock| {
                        indirect1[current_indirect2_data_blocks_idx1 as usize] = allocator.alloc();
                    });
                    current_indirect2_data_blocks_idx1 += 1;
                    if current_indirect2_data_blocks_idx1 == INDIRECT1_COUNT as u32 {
                        current_indirect2_data_blocks_idx1 = 0;
                        current_indirect2_data_blocks_idx0 += 1;
                    }
                }
            });
    }

    pub fn trunc(
        &mut self,
        new_size: u32,
        allocator: &DataBlockAllocator,
        device: &Arc<dyn BlockDevice>,
    ) {
        assert!(
            new_size <= self.size,
            "new size must be smaller than old size"
        );
        let current_data_blocks = self.data_blocks();
        let new_data_blocks = Self::needed_data(new_size);
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
            cache_entry(self.indirect2, Arc::clone(device))
                .lock()
                .modify(|indirect2: &mut IndexBlock| {
                    while idx0 > to_idx0 && idx0 == to_idx0 || idx1 > to_idx1 {
                        if idx1 == 0u32 {
                            idx1 = INDEX_ENTRY_COUNT as u32 - 1;
                            idx0 -= 1;
                        } else {
                            idx1 -= 1;
                        }
                        cache_entry(indirect2[idx0 as usize], Arc::clone(device))
                            .lock()
                            .modify(|indirect1: &mut IndexBlock| {
                                allocator.dealloc(indirect1[idx1 as usize]);
                                indirect1[idx1 as usize] = NULL;
                            });
                        if idx1 == 0 {
                            allocator.dealloc(indirect2[idx0 as usize]);
                            indirect2[idx0 as usize] = NULL;
                        }
                    }
                });
            if (to_idx0, to_idx1) == (0u32, 0u32) {
                allocator.dealloc(self.indirect2);
                self.indirect2 = NULL;
            }
        }

        //回收一级间接索引块管辖的数据块
        if current_data_blocks > INODE_DIRECT_COUNT as u32 {
            let mut idx =
                (current_data_blocks - INODE_DIRECT_COUNT as u32).min(INDEX_ENTRY_COUNT as u32);
            let to = new_data_blocks.max(INODE_DIRECT_COUNT as u32) - INODE_DIRECT_COUNT as u32;
            cache_entry(self.indirect1, Arc::clone(&device))
                .lock()
                .modify(|indirect1: &mut IndexBlock| {
                    while idx > to {
                        idx -= 1;
                        allocator.dealloc(indirect1[idx as usize]);
                        indirect1[idx as usize] = NULL;
                    }
                });
            if to == 0u32 {
                allocator.dealloc(self.indirect1);
                self.indirect1 = NULL;
            }
        }

        let mut idx = current_data_blocks.min(INODE_DIRECT_COUNT as u32);
        let to = new_data_blocks.min(INODE_DIRECT_COUNT as u32);
        while idx > to {
            idx -= 1;
            allocator.dealloc(self.direct[idx as usize]);
            self.direct[idx as usize] = NULL;
        }
    }

    pub fn clear(&mut self, allocator: &DataBlockAllocator, device: &Arc<dyn BlockDevice>) {
        self.trunc(0u32, allocator, device)
    }

    pub fn read(&self, mut from: u32, buf: &mut [u8], device: &Arc<dyn BlockDevice>) -> u32 {
        let end = (from + buf.len() as u32).min(self.size);
        if end <= from {
            return 0;
        }
        let mut read = 0u32;
        loop {
            let (logical_block, block_offset) =
                (from / BLOCK_SIZE as u32, from % BLOCK_SIZE as u32);
            let physical_block = self.nth_data_block(logical_block as usize, device);
            let this_cpy_end = ((logical_block + 1) * BLOCK_SIZE as u32).min(end);
            let this_cpy_size = this_cpy_end - from;

            let dest = &mut buf[read as usize..(read + this_cpy_size) as usize];
            cache_entry(physical_block, device.clone())
                .lock()
                .read(|block: &Block| {
                    let src =
                        &block[block_offset as usize..(block_offset + this_cpy_size) as usize];
                    dest.copy_from_slice(src);
                });
            read += this_cpy_size;

            if this_cpy_end == end {
                break;
            }
            from = this_cpy_end;
        }
        read
    }

    // 不会增长文件大小, 对于超出文件大小的写操作会被忽略
    pub fn write(&mut self, mut from: u32, buf: &[u8], device: &Arc<dyn BlockDevice>) -> u32 {
        let end = (from + buf.len() as u32).min(self.size);
        if end <= from {
            return 0;
        }
        let mut write = 0u32;
        loop {
            let (logical_block, block_offset) =
                (from / BLOCK_SIZE as u32, from % BLOCK_SIZE as u32);
            let physical_block = self.nth_data_block(logical_block as usize, device);
            let this_cpy_end = ((logical_block + 1) * BLOCK_SIZE as u32).min(end);
            let this_cpy_size = this_cpy_end - from;

            let src = &buf[write as usize..(write + this_cpy_size) as usize];
            cache_entry(physical_block, device.clone())
                .lock()
                .modify(|block: &mut Block| {
                    let dest =
                        &mut block[block_offset as usize..(block_offset + this_cpy_size) as usize];
                    dest.copy_from_slice(src);
                });
            write += this_cpy_size;
            if this_cpy_end == end {
                break;
            }
            from = this_cpy_end;
        }
        write
    }

    pub fn write_may_grow(
        &mut self,
        from: u32,
        buf: &[u8],
        device: &Arc<dyn BlockDevice>,
        allocator: &DataBlockAllocator,
    ) -> u32 {
        if self.size < from + buf.len() as u32 {
            self.grow(from + buf.len() as u32, allocator, device)
        }
        self.write(from, buf, device)
    }

    pub fn append(
        &mut self,
        buf: &[u8],
        device: &Arc<dyn BlockDevice>,
        allocator: &DataBlockAllocator,
    ) -> u32 {
        let size = self.size;
        self.grow(size + buf.len() as u32, allocator, device);
        self.write(size, buf, device)
    }
}

pub trait Directory {
    fn dir_insert(
        &mut self,
        to_insert: DirEntry,
        device: &Arc<dyn BlockDevice>,
        allocator: &DataBlockAllocator,
    );
    fn dir_entries(&self, device: &Arc<dyn BlockDevice>) -> Vec<DirEntry>;
    fn dir_delete(&mut self, name: &str, device: &Arc<dyn BlockDevice>) -> Option<DirEntry>;
    fn dir_find(&self, name: &str, device: &Arc<dyn BlockDevice>) -> Option<DirEntry>;
}

impl Directory for Inode {
    fn dir_insert(
        &mut self,
        to_insert: DirEntry,
        device: &Arc<dyn BlockDevice>,
        allocator: &DataBlockAllocator,
    ) {
        self.assert_dir();
        self.append(to_insert.as_bytes(), device, allocator);
    }

    fn dir_entries(&self, device: &Arc<dyn BlockDevice>) -> Vec<DirEntry> {
        DirectoryIterator::new(self, device.clone())
            .map(|(_, _, entry)| entry)
            .filter(|entry| entry.valid)
            .collect()
    }

    fn dir_delete(&mut self, name: &str, device: &Arc<dyn BlockDevice>) -> Option<DirEntry> {
        match DirectoryIterator::new(self, device.clone())
            .find(|(_, _, entry)| entry.name() == name && entry.valid)
        {
            Some((block, offset, entry)) => {
                cache_entry(block, device.clone())
                    .lock()
                    .modify(|block: &mut DirEntryBlock| {
                        block.get_mut(offset as usize).unwrap().valid = false
                    });
                Some(entry)
            }
            None => None,
        }
    }

    fn dir_find(&self, name: &str, device: &Arc<dyn BlockDevice>) -> Option<DirEntry> {
        DirectoryIterator::new(self, device.clone())
            .map(|(_, _, entry)| entry)
            .find(|entry| entry.valid && entry.name() == name)
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

struct DirectoryIterator {
    inode: *const Inode,
    logical_block: u32,
    block_offset: u32,
    block: DirEntryBlock,
    device: Arc<dyn BlockDevice>,
}

impl Iterator for DirectoryIterator {
    type Item = (BlockAddr, u32, DirEntry);

    fn next(&mut self) -> Option<Self::Item> {
        if self.block_offset * size_of::<DirEntry>() as u32 + self.logical_block * BLOCK_SIZE as u32
            >= self.inode().size
        {
            return None;
        }
        let physical_block = self
            .inode()
            .nth_data_block(self.logical_block as usize, &self.device);
        let block_offset = self.block_offset;
        if block_offset == 0 {
            cache_entry(physical_block, Arc::clone(&self.device))
                .lock()
                .read(|block: &DirEntryBlock| self.block = *block);
        }
        let ret = self.block[block_offset as usize];
        self.block_offset = block_offset + 1;
        if self.block_offset == DIR_ENTRY_COUNT as u32 {
            self.block_offset = 0;
            self.logical_block += 1;
        }
        Some((physical_block, block_offset, ret))
    }
}

impl DirectoryIterator {
    fn new(inode: &Inode, device: Arc<dyn BlockDevice>) -> Self {
        Self {
            inode,
            logical_block: 0,
            block_offset: 0,
            block: [Default::default(); DIR_ENTRY_COUNT],
            device,
        }
    }

    fn inode(&self) -> &Inode {
        unsafe { &*self.inode }
    }
}
