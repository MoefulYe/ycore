#![allow(unused)]

use crate::constant::{PPN_MASK, PPN_WIDTH};

use super::{
    address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum},
    frame_alloc::ALLOCATOR,
    virt_mem_area::Permission as VMAPermission,
};
use alloc::{string::String, vec::Vec};
use bitflags::*;
use log::{debug, info};

bitflags! {
    pub struct PTEFlags: u8 {
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

impl From<VMAPermission> for PTEFlags {
    fn from(perm: VMAPermission) -> Self {
        PTEFlags::from_bits(perm.bits()).unwrap()
    }
}

#[derive(Clone, Copy)]
pub struct PageTableEntry(pub usize);

//64 reserved 54 pyhs_pager_num 10 rsw 8 DAGUEWRV 0
impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        Self(ppn.0 << 10 | flags.bits() as usize)
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
        (self.0 >> 10).into()
    }

    pub fn flags(self) -> PTEFlags {
        unsafe { PTEFlags::from_bits_retain(self.0 as u8) }
    }

    pub fn is_valid(self) -> bool {
        self.flags().contains(PTEFlags::VAILD)
    }
}

#[derive(Clone, Copy)]
pub struct TopLevelEntry(pub PhysPageNum);

impl TopLevelEntry {
    pub fn token(&self) -> usize {
        8usize << 60 | self.0 .0
    }

    pub fn drop_page_table(self) {
        Self::_drop(self.0, 0);
    }

    fn _drop(ppn: PhysPageNum, depth: u8) {
        if depth != 2 {
            ppn.read_as_page_table()
                .iter()
                .filter(|entry| entry.is_valid())
                .map(|entry| entry.ppn())
                .for_each(|ppn| ALLOCATOR.exclusive_access().dealloc(ppn))
        } else {
            ALLOCATOR.exclusive_access().dealloc(ppn)
        }
    }

    //手动回收页表管理的物理页帧
    pub fn drop(self) {
        Self::_drop(self.0, 0);
    }

    pub fn new() -> Self {
        let frame = ALLOCATOR.exclusive_access().alloc();
        Self(frame)
    }

    pub fn from_ppn(ppn: PhysPageNum) -> Self {
        Self(ppn)
    }

    pub fn from_token(stap: usize) -> Self {
        Self(PhysPageNum(stap & PPN_MASK))
    }

    pub fn map(self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_or_create(vpn);
        *pte = PageTableEntry::new(ppn, PTEFlags::VAILD | flags);
    }

    pub fn unmap(self, vpn: VirtPageNum) {
        if let Some(pte) = self.find_pte(vpn) {
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
        None
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

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }

    pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        let (vpn, offset) = va.split();
        self.translate(vpn)
            .map(|entry| entry.ppn().phys_addr(offset))
    }

    pub fn translate_virt_str(self, virt_str: *const u8) -> String {
        unsafe {
            let mut s = Vec::new();
            let mut va = VirtAddr(virt_str as usize);
            loop {
                let c = *(self.translate_va(va).unwrap().0 as *const u8);
                if c == 0 {
                    break;
                } else {
                    s.push(c);
                    va.0 += 1;
                }
            }
            String::from_utf8_unchecked(s)
        }
    }

    //要求对象的内存布局不能跨页
    pub fn translate_virt_ref<T>(self, ptr: *const T) -> &'static T {
        self.translate_va(VirtAddr(ptr as usize)).unwrap().as_ref()
    }

    pub fn translate_virt_mut<T>(self, ptr: *mut T) -> &'static mut T {
        self.translate_va(VirtAddr(ptr as usize)).unwrap().as_mut()
    }
}
