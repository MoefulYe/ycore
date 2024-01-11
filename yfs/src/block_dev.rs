use crate::constant::BlockAddr;
use core::any::Any;
use core::fmt::Debug;

pub trait BlockDevice: Send + Sync + Any + Debug {
    fn read_block(&self, block_addr: BlockAddr, buf: &mut [u8]);
    fn write_block(&self, block_addr: BlockAddr, buf: &[u8]);
}
