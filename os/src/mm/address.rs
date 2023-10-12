use crate::constant::{PAGE_SIZE, PAGE_SIZE_BITS, PTES_NUM};

use super::page_table::PageTableEntry;

const PA_WIDTH: usize = 56;
const VA_WIDTH: usize = 39;
const PPN_WIDTH: usize = PA_WIDTH - PAGE_SIZE_BITS;
const VPN_WIDTH: usize = VA_WIDTH - PAGE_SIZE_BITS;

//56位 符号拓展
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysAddr(pub usize);

impl PhysAddr {
    pub const NULL: PhysAddr = PhysAddr(0);
    pub fn phys_page_num(self) -> PhysPageNum {
        PhysPageNum(self.0 >> PAGE_SIZE_BITS)
    }

    pub fn page_offset(self) -> usize {
        self.0 & (1 << PAGE_SIZE_BITS - 1)
    }

    pub fn split(self) -> (PhysPageNum, usize) {
        (self.phys_page_num(), self.page_offset())
    }
}

impl From<usize> for PhysAddr {
    fn from(v: usize) -> Self {
        Self(v & (1 << PA_WIDTH - 1))
    }
}

//39位 符号拓展
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(pub usize);

impl VirtAddr {
    pub const NULL: VirtAddr = VirtAddr(0);
    pub fn virt_page_num(self) -> VirtPageNum {
        VirtPageNum(self.0 >> PAGE_SIZE_BITS)
    }

    pub fn page_offset(self) -> usize {
        self.0 & (1 << PAGE_SIZE_BITS - 1)
    }

    pub fn split(self) -> (VirtPageNum, usize) {
        (self.virt_page_num(), self.page_offset())
    }

    pub fn vpn<const I: usize>(self) -> usize {
        match I {
            0 => self.0 >> 30 & 0x1ff,
            1 => self.0 >> 21 & 0x1ff,
            2 => self.0 >> 12 & 0x1ff,
            _ => 0,
        }
    }

    pub fn vpns(self) -> [usize; 3] {
        [self.vpn::<0>(), self.vpn::<1>(), self.vpn::<2>()]
    }
}

impl From<usize> for VirtAddr {
    fn from(v: usize) -> Self {
        Self(v & (1 << VA_WIDTH - 1))
    }
}

//低44位有效
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysPageNum(pub usize);

impl PhysPageNum {
    pub const NULL: PhysPageNum = PhysPageNum(0);
    pub fn phys_addr(self, offset: usize) -> PhysAddr {
        PhysAddr((self.0 << PAGE_SIZE_BITS) | (offset & (1 << PAGE_SIZE_BITS - 1)))
    }

    pub fn floor(self) -> PhysAddr {
        PhysAddr(self.0 << PAGE_SIZE_BITS)
    }

    pub fn ceil(self) -> PhysAddr {
        PhysAddr((self.0 + 1) << PAGE_SIZE_BITS)
    }

    pub fn clear(self) -> Self {
        unsafe { core::slice::from_raw_parts_mut(self.floor().0 as *mut u8, PAGE_SIZE).fill(0) }
        self
    }

    pub fn read_as_page_table(self) -> &'static mut [PageTableEntry; PTES_NUM] {
        unsafe { &mut *(self.floor().0 as *mut [PageTableEntry; PTES_NUM]) }
    }

    pub fn read_as_bytes_array(self) -> &'static mut [u8; PAGE_SIZE] {
        unsafe { &mut *(self.floor().0 as *mut [u8; PAGE_SIZE]) }
    }

    pub fn read_as<T>(self) -> &'static mut T {
        unsafe { &mut *(self.floor().0 as *mut T) }
    }
}

impl From<usize> for PhysPageNum {
    fn from(v: usize) -> Self {
        Self(v & (1 << PPN_WIDTH - 1))
    }
}

//低27位有效
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtPageNum(pub usize);

impl VirtPageNum {
    const NULL: VirtPageNum = VirtPageNum(0);
    pub fn virt_addr(self, offset: usize) -> VirtAddr {
        VirtAddr((self.0 << PAGE_SIZE_BITS) | (offset & (1 << PAGE_SIZE_BITS - 1)))
    }

    pub fn floor(self) -> VirtAddr {
        VirtAddr(self.0 << PAGE_SIZE_BITS)
    }

    pub fn ceil(self) -> VirtAddr {
        VirtAddr((self.0 + 1) << PAGE_SIZE_BITS)
    }

    pub fn indexs(self) -> [usize; 3] {
        [self.0 >> 18 & 0x1ff, self.0 >> 9 & 0x1ff, self.0 & 0x1ff]
    }
}

impl From<usize> for VirtPageNum {
    fn from(v: usize) -> Self {
        Self(v & (1 << VPN_WIDTH - 1))
    }
}
