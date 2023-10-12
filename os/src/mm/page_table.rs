use super::{
    address::{PhysPageNum, VirtPageNum},
    frame_allocator::ALLOCATOR,
};
use bitflags::*;

bitflags! {
    pub struct PTEFlags: usize {
        const VAILD = 1 << 0;
        const READ = 1 << 1;
        const WRITE = 1 << 2;
        const EXEC = 1 << 3;
        const USER = 1 << 4;
        const GLOBAL = 1 << 5;
        const ACCESSED = 1 << 6;
        const DIRTY = 1 << 7;
    }
}

#[derive(Clone, Copy)]
pub struct PageTableEntry(pub usize);

//64 reserved 54 pyhs_pager_num 10 rsw 8 DAGUEWRV 0
impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        Self(ppn.0 << 10 | flags.bits as usize)
    }

    pub fn new_valid(ppn: PhysPageNum) -> Self {
        Self::new(ppn, PTEFlags::VAILD)
    }

    pub fn empty() -> Self {
        Self(0)
    }

    pub fn split(self) -> (PhysPageNum, PTEFlags) {
        (self.ppn(), self.flags())
    }

    pub fn ppn(self) -> PhysPageNum {
        (self.0 >> 10 & (1usize << 44 - 1)).into()
    }

    pub fn flags(self) -> PTEFlags {
        unsafe { PTEFlags::from_bits_unchecked(self.0 & 0b1111_1111) }
    }

    pub fn is_valid(self) -> bool {
        self.flags().contains(PTEFlags::VAILD)
    }
}

pub struct TopLevelEntry(PhysPageNum);

impl Drop for TopLevelEntry {
    fn drop(&mut self) {
        Self::_drop(self.0, 0);
    }
}

impl TopLevelEntry {
    // 回收物理页号指向的页,考虑到多级页表情况,物理页构成一颗深度为4的树, 所以递归回收, D代表递归深度
    fn _drop(ppn: PhysPageNum, depth: usize) {
        if depth == 3 {
            //物理页号指向了非页表节点 即叶子节点
            ALLOCATOR.exclusive_access().dealloc(ppn);
        } else {
            let depth = depth + 1;
            ppn.read_as_page_table()
                .iter()
                .filter(|pte| pte.is_valid())
                .for_each(|item| Self::_drop(item.ppn(), depth));
            //回收自己本身
            ALLOCATOR.exclusive_access().dealloc(ppn);
        }
    }

    pub fn new() -> Self {
        let frame = ALLOCATOR.exclusive_access().alloc();
        Self(frame)
    }

    pub fn with_ppn(ppn: PhysPageNum) -> Self {
        Self(ppn)
    }

    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_or_create(vpn);
        *pte = PageTableEntry::new_valid(ppn);
    }

    pub fn unmap(&mut self, vpn: VirtPageNum) {
        if let Some(pte) = self.find_pte(vpn) {
            let ppn = pte.ppn();
            ALLOCATOR.exclusive_access().dealloc(ppn);
            *pte = PageTableEntry::empty();
        } else {
            panic!("unmap a unmapped page")
        }
    }

    pub fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let indexs = vpn.indexs();
        let mut ppn = self.0;
        for i in 0..3 {
            let pte = unsafe { ppn.read_as_page_table().get_unchecked_mut(indexs[i]) };
            if i == 2 {
                return Some(pte);
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        //unreachable
        return None;
    }

    //在查询路径上找不到页表项时,创建一个新的页表项
    pub fn find_pte_or_create(&self, vpn: VirtPageNum) -> &mut PageTableEntry {
        let indexs = vpn.indexs();
        let mut ppn = self.0;
        for i in 0..3 {
            let pte = unsafe { ppn.read_as_page_table().get_unchecked_mut(indexs[i]) };
            if i == 2 {
                return pte;
            }
            if !pte.is_valid() {
                let frame = ALLOCATOR.exclusive_access().alloc();
                *pte = PageTableEntry::new_valid(frame);
            }
            ppn = pte.ppn();
        }
        unreachable!();
    }
}
