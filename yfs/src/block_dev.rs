use crate::constant::BlockAddr;
use core::any::Any;

pub trait BlockDevice: Send + Sync + Any {
    fn read_block(&self, block_addr: BlockAddr, buf: &mut [u8]);
    fn write_block(&self, block_addr: BlockAddr, buf: &[u8]);
}
