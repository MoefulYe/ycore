use core::mem::size_of;

use alloc::sync::Arc;

use crate::{constant::{BlockAddr, BLOCK_SIZE, BLOCK_BITS, NULL, Block}, block_dev::BlockDevice, block_cache::BLOCK_CACHE};

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


const INODE_DIRECT_COUNT: u32 = 28;
const INDIRECT1_COUNT: u32 = BLOCK_BITS / 4;
const INDIRECT2_COUNT: u32 = INDIRECT1_COUNT * INDIRECT1_COUNT;
const INDIRECT1_BOUND: u32 = INDIRECT1_COUNT + INODE_DIRECT_COUNT;
const INDIRECT2_BOUND: u32 = INDIRECT2_COUNT + INDIRECT1_BOUND;
type IndexBlock = [BlockAddr; BLOCK_SIZE/size_of<BlockAddr>()];

#[repr(C)]
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
        if n < INODE_DIRECT_COUNT {
            return self.direct[n]
        } else if n < INDIRECT1_BOUND {
            BLOCK_CACHE
                .lock()
                .get_cache(self.indirect1, Arc::clone(device))
                .lock()
                .read(0, |block: &Block| block[n - INODE_DIRECT_COUNT])
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InodeType {
    File,
    Dir,
}
