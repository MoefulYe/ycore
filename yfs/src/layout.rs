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
