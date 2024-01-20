use virtio_drivers::{Hal, VirtIOBlk, VirtIOHeader};
use yfs::{block_dev::BlockDevice, constant::BlockAddr};

use crate::{
    mm::{
        address::{PhysAddr, PhysPageNum, VirtAddr},
        frame_alloc::ALLOCATOR,
        mem_set::kernel_token,
        page_table::TopLevelEntry,
    },
    sync::up::UPSafeCell,
};

const VIRTIO0: usize = 0x1000_1000;

pub struct VirtIOBlock(UPSafeCell<VirtIOBlk<'static, VirtioHal>>);

impl VirtIOBlock {
    pub fn new() -> Self {
        unsafe {
            Self(UPSafeCell::new(
                VirtIOBlk::<VirtioHal>::new(&mut *(VIRTIO0 as *mut VirtIOHeader))
                    .expect("virtio_blk: failed to init virtio_blk"),
            ))
        }
    }
}

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_addr: BlockAddr, buf: &mut [u8]) {
        self.0
            .exclusive_access()
            .read_block(block_addr as usize, buf)
            .expect("virtio_blk: read_block failed")
    }

    fn write_block(&self, block_addr: BlockAddr, buf: &[u8]) {
        self.0
            .exclusive_access()
            .write_block(block_addr as usize, buf)
            .expect("virtio_blk: write_block failed")
    }
}

pub struct VirtioHal;

impl Hal for VirtioHal {
    fn dma_alloc(pages: usize) -> virtio_drivers::PhysAddr {
        let mut base = PhysPageNum(0);
        for i in 0..pages {
            let frame = ALLOCATOR.exclusive_access().alloc();
            if i == 0 {
                base = frame;
            }
            assert_eq!(base + i, frame)
        }
        base.floor().raw()
    }

    fn dma_dealloc(paddr: virtio_drivers::PhysAddr, pages: usize) -> i32 {
        let base = PhysAddr(paddr).phys_page_num();
        (0..pages).for_each(|i| ALLOCATOR.exclusive_access().dealloc(base + i));
        0
    }

    fn phys_to_virt(paddr: virtio_drivers::PhysAddr) -> virtio_drivers::VirtAddr {
        paddr
    }

    fn virt_to_phys(vaddr: virtio_drivers::VirtAddr) -> virtio_drivers::PhysAddr {
        TopLevelEntry::from_token(kernel_token())
            .translate_va(VirtAddr(vaddr))
            .expect("virtio_blk: virt_to_phys failed")
            .raw()
    }
}
