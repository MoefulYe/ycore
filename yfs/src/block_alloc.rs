use crate::constant::BlockAddr;

pub trait DataBlockAlloc {
    fn alloc(&mut self) -> BlockAddr;
    fn dealloc(&mut self, block_addr: BlockAddr);
}

pub trait InodeBlockAlloc {
    fn alloc(&mut self) -> BlockAddr;
    fn dealloc(&mut self, block_addr: BlockAddr);
}
