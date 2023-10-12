use core::ops::Range;

use alloc::vec::Vec;

use super::{
    address::{VPNRange, VirtPageNum},
    page_table::TopLevelEntry,
    virt_mem_area::{MapType, Permission, VirtMemArea},
};

pub struct MemSet {
    entry: TopLevelEntry,
    vmas: Vec<VirtMemArea>,
}

impl MemSet {
    pub fn new_bare() -> Self {
        Self {
            entry: TopLevelEntry::new(),
            vmas: Vec::new(),
        }
    }

    //创建一个逻辑上的虚拟内存段后(此时虚拟页还没有映射到物理内存页上), 把虚拟内存段挂载到MemSet上并建立映射关系
    pub fn push_vma(&mut self, mut vma: VirtMemArea) {
        vma.map(self.entry);
        self.vmas.push(vma);
    }

    pub fn push_vma_with_data(&mut self, mut vma: VirtMemArea, src: &[u8]) {
        vma.map(self.entry);
        vma.memcpy(self.entry, src);
        self.vmas.push(vma);
    }

    //调用者要保证和已存在的vma不冲突
    pub fn insert_framed_area(&mut self, range: Range<VirtPageNum>, perm: Permission) {
        self.push_vma(VirtMemArea::new(range, MapType::Framed, perm))
    }

    // pub fn new_kernel() -> Self;
    // pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize);
}
