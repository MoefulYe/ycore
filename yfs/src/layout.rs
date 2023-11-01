use core::mem::size_of;

use alloc::sync::Arc;
use spin::Mutex;

const INODE_DIRECT_COUNT: usize = 28;
const INDEX_BLOCK_COUNT: usize = BLOCK_SIZE / size_of::<BlockAddr>();
const INDIRECT1_COUNT: usize = INDEX_BLOCK_COUNT;
const INDIRECT2_COUNT: usize = INDEX_BLOCK_COUNT * INDEX_BLOCK_COUNT;
const INDIRECT1_BOUND: usize = INDIRECT1_COUNT + INODE_DIRECT_COUNT;
const INDIRECT2_BOUND: usize = INDIRECT2_COUNT + INDIRECT1_BOUND;
// 最大支持的文件大小大概是0x813800bytes， 大概是8MB
const MAX_FILE_SIZE: u32 =
    (INODE_DIRECT_COUNT + INDIRECT1_COUNT + INDIRECT2_COUNT) as u32 * BLOCK_SIZE as u32;
pub type IndexBlock = [BlockAddr; BLOCK_SIZE / size_of::<BlockAddr>()];
pub type DataBlock = Block;
pub type INodeBlock = [INode; BLOCK_SIZE / size_of::<INode>()];

use crate::{
    block_alloc::DataBlockAlloc,
    block_cache::BLOCK_CACHE,
    block_dev::BlockDevice,
    constant::{Block, BlockAddr, BLOCK_SIZE, NULL},
};

#[repr(C)]
pub struct SuperBlock {
    pub magic: u32,
    pub total_cnt: u32,
    pub inode_bitmap_cnt: u32,
    pub inode_area_cnt: u32,
    pub data_bitmap_cnt: u32,
    pub data_area_cnt: u32,
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
        }
    }

    pub fn init(
        &mut self,
        total_cnt: u32,
        inode_bitmap_cnt: u32,
        inode_area_cnt: u32,
        data_bitmap_cnt: u32,
        data_area_cnt: u32,
    ) {
        self.magic = Self::MAGIC;
        self.total_cnt = total_cnt;
        self.inode_bitmap_cnt = inode_bitmap_cnt;
        self.inode_area_cnt = inode_area_cnt;
        self.data_bitmap_cnt = data_bitmap_cnt;
        self.data_area_cnt = data_area_cnt;
    }

    pub fn valid(&self) -> bool {
        self.magic == Self::MAGIC
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum INodeType {
    File,
    Dir,
}

#[repr(C)]
pub struct INode {
    pub size: u32,
    pub direct: [u32; INODE_DIRECT_COUNT],
    pub indirect1: u32,
    pub indirect2: u32,
    inode_type: INodeType,
}

impl INode {
    pub fn init(&mut self, inode_type: INodeType) {
        self.size = 0;
        self.inode_type = inode_type;
        self.direct = [NULL; INODE_DIRECT_COUNT];
        self.indirect1 = NULL;
        self.indirect2 = NULL;
    }

    pub fn is_file(&self) -> bool {
        self.inode_type == INodeType::File
    }

    pub fn is_dir(&self) -> bool {
        self.inode_type == INodeType::Dir
    }

    pub fn nth_data_block(&self, n: u32, device: &Arc<dyn BlockDevice>) -> BlockAddr {
        if n < INODE_DIRECT_COUNT as u32 {
            let ret = self.direct[n as usize];
            assert!(ret != NULL, "unexpected NULL block");
            ret
        } else if n < INDIRECT1_BOUND as u32 {
            assert!(self.indirect1 != NULL, "unexpected NULL block");
            let ret = { BLOCK_CACHE.lock().entry(self.indirect1, Arc::clone(device)) }
                .lock()
                .read(|block: &IndexBlock| block[n as usize - INODE_DIRECT_COUNT]);
            assert!(ret != NULL, "unexpected NULL block");
            ret
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
            let ret = { BLOCK_CACHE.lock().entry(indirect1, Arc::clone(device)) }
                .lock()
                .read(|block: &IndexBlock| block[idx1 as usize]);
            assert!(ret != NULL, "unexpected NULL block");
            ret
        } else {
            panic!("invalid block index");
        }
    }

    //分配一个size大小的文件需要多少数据块
    fn needed_data_blocks(size: u32) -> u32 {
        (size + BLOCK_SIZE as u32 - 1) / BLOCK_SIZE as u32
    }

    //分配一个size大小的文件需要多少块(包括索引快)
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
            total += (data_blocks - INDIRECT1_BOUND as u32 + INDEX_BLOCK_COUNT as u32 - 1)
                / INDEX_BLOCK_COUNT as u32;
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
        assert!(
            new_size >= self.size,
            "new_size must be larger than old size"
        );
        Self::needed_total_blocks(new_size) - Self::needed_total_blocks(self.size)
    }

    //为inode分配新的数据块来适应新的文件大小
    //但不改变inode的size
    pub fn alloc_new_block(
        &mut self,
        new_size: u32,
        allocator: &mut impl DataBlockAlloc,
        device: &Arc<dyn BlockDevice>,
    ) {
        assert!(
            new_size >= self.size,
            "new_size must be larger than old size"
        );
        let mut current_data_blocks = self.data_blocks();
        let new_data_blocks = Self::needed_data_blocks(new_size);
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
        { BLOCK_CACHE.lock().entry(self.indirect1, Arc::clone(device)) }
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

        { BLOCK_CACHE.lock().entry(self.indirect2, Arc::clone(device)) }
            .lock()
            .modify(|indirect2: &mut IndexBlock| {
                while current_indirect2_data_blocks_idx0 < new_indirect2_data_blocks_idx0
                    || current_indirect2_data_blocks_idx0 == new_indirect2_data_blocks_idx0
                        && current_indirect2_data_blocks_idx1 < new_indirect2_data_blocks_idx1
                {
                    //现在current指向了新的一级索引块, 所以要分配一级索引块
                    if current_indirect2_data_blocks_idx1 == 0 {
                        indirect2[current_indirect2_data_blocks_idx0 as usize] = allocator.alloc();
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

    //释放所有数据块并将inode的size置为0
    pub fn clear(&mut self, device: &Arc<dyn BlockDevice>, allocator: &mut impl DataBlockAlloc) {
        let total = self.data_blocks();
        self.size = 0;
        let mut current = 0;
        //释放直接索引的数据块
        while current < total.min(INODE_DIRECT_COUNT as u32) {
            allocator.dealloc(self.direct[current as usize]);
            self.direct[current as usize] = NULL;
            current += 1;
        }
        if total <= INODE_DIRECT_COUNT as u32 {
            return;
        }

        //释放一级间接索引块和其管辖的数据块
        let indirect_blocks_num = total - INODE_DIRECT_COUNT as u32;
        let mut current = 0;
        { BLOCK_CACHE.lock().entry(self.indirect1, Arc::clone(device)) }
            .lock()
            .modify(|indirect1: &mut IndexBlock| {
                while current < indirect_blocks_num.min(INDIRECT1_COUNT as u32) {
                    allocator.dealloc(indirect1[current as usize]);
                    // indirect1[current as usize] = NULL;
                    current += 1;
                }
            });
        allocator.dealloc(self.indirect1);
        self.indirect1 = NULL;
        if total <= INDIRECT1_BOUND as u32 {
            return;
        }
        //释放二级间接索引块和其管辖的数据块
        let indirect2_blocks_num = indirect_blocks_num - INDIRECT1_COUNT as u32;
        let idx0 = indirect2_blocks_num / INDEX_BLOCK_COUNT as u32;
        let idx1 = indirect2_blocks_num % INDEX_BLOCK_COUNT as u32;
        { BLOCK_CACHE.lock().entry(self.indirect2, Arc::clone(device)) }
            .lock()
            .modify(|indirect2: &mut IndexBlock| {
                for &mut entry in indirect2.iter_mut().take(idx0 as usize) {
                    { BLOCK_CACHE.lock().entry(entry, Arc::clone(device)) }
                        .lock()
                        .modify(|indirect1: &mut IndexBlock| {
                            indirect1
                                .iter_mut()
                                .for_each(|&mut entry| allocator.dealloc(entry))
                        });
                    allocator.dealloc(entry);
                }

                if idx1 > 0 {
                    {
                        BLOCK_CACHE
                            .lock()
                            .entry(indirect2[idx0 as usize], Arc::clone(device))
                    }
                    .lock()
                    .modify(|indirect1: &mut IndexBlock| {
                        for &mut entry in indirect1.iter_mut().take(idx1 as usize) {
                            allocator.dealloc(entry);
                        }
                    });
                    allocator.dealloc(indirect2[idx0 as usize]);
                }
            });
        allocator.dealloc(self.indirect2);
        self.indirect2 = NULL;
    }

    pub fn read_from(&self, offset: usize, buf: &mut [u8], device: &Arc<dyn BlockDevice>) {}
}

enum SeekFrom {
    Start(u32),
    End(i32),
    Cur(i32),
}

struct FileDataIter {
    inode: *mut INode,
    n: u32,
    block_addr: BlockAddr,
    offset: u32,
    device: Arc<dyn BlockDevice>,
}

impl FileDataIter {
    fn file_size(&self) -> u32 {
        unsafe { (*self.inode).size }
    }

    fn new(inode: &mut INode, device: Arc<dyn BlockDevice>) -> Self {
        Self {
            inode: inode as *mut _,
            n: 0,
            offset: 0,
            block_addr: inode.nth_data_block(0, &device),
            device,
        }
    }

    //还没有定位到文件首个数据块, 使用的时候需要先调用一次seek
    unsafe fn unalloc(inode: &mut INode, device: Arc<dyn BlockDevice>) -> Self {
        Self {
            inode: inode as *mut _,
            n: 0,
            offset: 0,
            block_addr: NULL,
            device,
        }
    }

    fn seek(&mut self, seek: SeekFrom) {
        unsafe {
            let to = match seek {
                SeekFrom::Start(to) => to,
                SeekFrom::End(step) => self.file_size() + step as u32,
                SeekFrom::Cur(step) => self.n * BLOCK_SIZE as u32 + self.offset + step as u32,
            };
            assert!(to < self.file_size(), "seek out of range");
            self.n = to / BLOCK_SIZE as u32;
            self.offset = to % BLOCK_SIZE as u32;
            self.block_addr = (*self.inode).nth_data_block(self.n, &self.device);
        }
    }

    fn read(&mut self, buf: &mut [u8]) -> u32 {
        let mut read = 0;
        while read < buf.len() && self.n < self.file_size() / BLOCK_SIZE as u32 {
            if self.offset == BLOCK_SIZE as u32 {
                self.n += 1;
                self.offset = 0;
                self.block_addr = unsafe { (*self.inode).nth_data_block(self.n, &self.device) };
            }
            let block = {
                BLOCK_CACHE
                    .lock()
                    .entry(self.block_addr, Arc::clone(&self.device))
            }
            .lock()
            .read(|block: &Block| &block[self.offset as usize..]);
            let to_read = buf.len() - read;
            let to_read = to_read.min(BLOCK_SIZE - self.offset as usize);
            buf[read..read + to_read].copy_from_slice(&block[..to_read]);
            read += to_read;
            self.offset += to_read as u32;
        }
        read
    }
}
