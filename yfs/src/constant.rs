pub type BlockAddr = u32;
pub type Block = [u8; BLOCK_SIZE];

pub const NULL: BlockAddr = 0;
pub const BLOCK_SIZE: usize = 512;
pub const BLOCK_BITS: usize = BLOCK_SIZE * 8;
