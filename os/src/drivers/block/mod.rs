use alloc::sync::Arc;
use lazy_static::lazy_static;
use virtio_blk::VirtIOBlock;
use yfs::block_dev::BlockDevice;

pub mod virtio_blk;

lazy_static! {
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = Arc::new(VirtIOBlock::new());
}
