pub type BlockAddr = u32;
pub type Block = [u8; BLOCK_SIZE];
pub type InodeAddr = (BlockAddr, u32);

pub fn inode2addr(inode: u32, inode_data_start: BlockAddr) -> InodeAddr {
    (inode >> 2 + inode_data_start, inode & 3)
}
pub fn addr2inode((block_addr, offset): InodeAddr, inode_data_start: BlockAddr) -> u32 {
    (block_addr - inode_data_start) << 2 + offset
}

pub const NULL: BlockAddr = 0;
pub const SUPER: BlockAddr = 0;
pub const BLOCK_SIZE: usize = 512;
pub const BLOCK_BITS: usize = BLOCK_SIZE * 8;
