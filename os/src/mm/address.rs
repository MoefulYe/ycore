#![allow(unused)]
use core::{
    fmt::Display,
    ops::{Add, AddAssign, Range, Sub, SubAssign},
};

use crate::constant::{
    PAGE_MASK, PAGE_SIZE, PAGE_SIZE_BITS, PA_MASK, PA_WIDTH, PPN_MASK, PPN_WIDTH, PTES_NUM,
    VA_MASK, VA_WIDTH, VPN_MASK, VPN_WIDTH,
};

use super::page_table::{PageTableEntry, TopLevelEntry};

//56位 符号拓展
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysAddr(pub usize);

impl Add<usize> for PhysAddr {
    type Output = PhysAddr;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Sub<usize> for PhysAddr {
    type Output = PhysAddr;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl Sub for PhysAddr {
    type Output = usize;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0 - rhs.0
    }
}

impl PhysAddr {
    pub const NULL: PhysAddr = PhysAddr(0);
    pub fn phys_page_num(self) -> PhysPageNum {
        PhysPageNum((self.0 >> PAGE_SIZE_BITS) & PPN_MASK)
    }

    pub fn page_offset(self) -> usize {
        self.0 & PAGE_MASK
    }

    pub fn split(self) -> (PhysPageNum, usize) {
        (self.phys_page_num(), self.page_offset())
    }

    pub fn as_ref<T>(self) -> &'static T {
        unsafe { &mut *(self.0 as *mut T) }
    }

    pub fn as_mut<T>(self) -> &'static mut T {
        unsafe { &mut *(self.0 as *mut T) }
    }
}

impl From<usize> for PhysAddr {
    fn from(v: usize) -> Self {
        Self(v & PA_MASK)
    }
}

//39位 符号拓展
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(pub usize);

impl Add<usize> for VirtAddr {
    type Output = VirtAddr;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Sub<usize> for VirtAddr {
    type Output = VirtAddr;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl Sub<VirtAddr> for VirtAddr {
    type Output = usize;

    fn sub(self, rhs: VirtAddr) -> Self::Output {
        self.0 - rhs.0
    }
}

impl VirtAddr {
    pub const NULL: VirtAddr = VirtAddr(0);
    pub fn virt_page_num(self) -> VirtPageNum {
        VirtPageNum((self.0 >> PAGE_SIZE_BITS) & VPN_MASK)
    }

    pub fn floor(self) -> VirtPageNum {
        VirtPageNum((self.0 >> PAGE_SIZE_BITS) & VPN_MASK)
    }
    pub fn ceil(self) -> VirtPageNum {
        if self.0 == 0 {
            VirtPageNum(0)
        } else {
            VirtPageNum(((self.0 - 1 + PAGE_SIZE) >> PAGE_SIZE_BITS) & VPN_MASK)
        }
    }

    pub fn page_offset(self) -> usize {
        self.0 & PAGE_MASK
    }

    pub fn split(self) -> (VirtPageNum, usize) {
        (self.virt_page_num(), self.page_offset())
    }

    pub fn raw<T>(self) -> *mut T {
        self.0 as *mut T
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
        Self(v & VA_MASK)
    }
}

impl From<u64> for VirtAddr {
    fn from(v: u64) -> Self {
        Self((v & VA_MASK as u64) as usize)
    }
}

//低44位有效
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysPageNum(pub usize);

impl Display for PhysPageNum {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl AddAssign<usize> for PhysPageNum {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl SubAssign<usize> for PhysPageNum {
    fn sub_assign(&mut self, rhs: usize) {
        self.0 -= rhs;
    }
}

impl Add<usize> for PhysPageNum {
    type Output = PhysPageNum;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Sub<usize> for PhysPageNum {
    type Output = PhysPageNum;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl Sub for PhysPageNum {
    type Output = usize;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0 - rhs.0
    }
}

impl PhysPageNum {
    pub const NULL: PhysPageNum = PhysPageNum(0);

    pub fn identical_map(self) -> VirtPageNum {
        VirtPageNum(self.0)
    }

    pub fn phys_addr(self, offset: usize) -> PhysAddr {
        PhysAddr((self.0 << PAGE_SIZE_BITS) | (offset & PAGE_MASK))
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
        Self(v & PPN_MASK)
    }
}

//低27位有效
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtPageNum(pub usize);

impl Display for VirtPageNum {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl AddAssign<usize> for VirtPageNum {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl SubAssign<usize> for VirtPageNum {
    fn sub_assign(&mut self, rhs: usize) {
        self.0 -= rhs;
    }
}

impl Add<usize> for VirtPageNum {
    type Output = VirtPageNum;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Sub<usize> for VirtPageNum {
    type Output = VirtPageNum;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl Sub for VirtPageNum {
    type Output = usize;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0 - rhs.0
    }
}

impl VirtPageNum {
    pub const NULL: VirtPageNum = VirtPageNum(0);
    pub fn identical_map(self) -> PhysPageNum {
        PhysPageNum(self.0)
    }

    pub fn virt_addr(self, offset: usize) -> VirtAddr {
        VirtAddr((self.0 << PAGE_SIZE_BITS) | (offset & PAGE_MASK))
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

    pub const fn sub(self, rhs: usize) -> Self {
        Self(self.0 - rhs)
    }
}

impl From<usize> for VirtPageNum {
    fn from(v: usize) -> Self {
        Self(v & VPN_MASK)
    }
}

#[derive(Clone, Copy)]
pub struct VPNRange {
    pub start: VirtPageNum,
    pub end: VirtPageNum,
}

impl VPNRange {
    pub fn identical_map(self) -> PPNRange {
        PPNRange {
            start: self.start.identical_map(),
            end: self.end.identical_map(),
        }
    }
}

impl VPNRange {
    pub fn new(range: Range<VirtPageNum>) -> Self {
        range.into()
    }

    pub fn size(&self) -> usize {
        (self.end - self.start) << PAGE_SIZE_BITS
    }
}

impl Iterator for VPNRange {
    type Item = VirtPageNum;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            None
        } else {
            let ret = self.start;
            self.start += 1;
            Some(ret)
        }
    }
}

impl From<Range<VirtPageNum>> for VPNRange {
    fn from(range: Range<VirtPageNum>) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

#[derive(Clone, Copy)]
pub struct PPNRange {
    pub start: PhysPageNum,
    pub end: PhysPageNum,
}

impl PPNRange {
    pub fn identical_map(self) -> VPNRange {
        VPNRange {
            start: self.start.identical_map(),
            end: self.end.identical_map(),
        }
    }
}

impl Iterator for PPNRange {
    type Item = PhysPageNum;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            None
        } else {
            let ret = self.start;
            self.start += 1;
            Some(ret)
        }
    }
}

impl From<Range<PhysPageNum>> for PPNRange {
    fn from(range: Range<PhysPageNum>) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

pub struct VirtBufIter {
    begin: VirtAddr,
    end: VirtAddr,
    page_table_entry: TopLevelEntry,
}

impl VirtBufIter {
    pub fn new(range: Range<VirtAddr>, page_table_entry: TopLevelEntry) -> Self {
        Self {
            begin: range.start,
            end: range.end,
            page_table_entry,
        }
    }
}

pub trait Reader<T> {
    fn read(&mut self, src: T) -> usize {
        0
    }
}

impl Reader<&[u8]> for VirtBufIter {
    fn read(&mut self, src: &[u8]) -> usize {
        let mut written = 0;
        for slice in self {
            let len = core::cmp::min(slice.len(), src.len() - written);
            slice[..len].copy_from_slice(&src[written..written + len]);
            written += len;
        }
        written
    }
}

impl Iterator for VirtBufIter {
    type Item = &'static mut [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.begin < self.end {
            let (start_page, start_offset) = self.begin.split();
            let (end_page, end_offset) = self.end.split();
            self.begin = start_page.ceil();
            let slice_begin = start_offset;
            let slice_end = if start_page == end_page {
                end_offset
            } else {
                PAGE_SIZE
            };
            let ppn = self.page_table_entry.translate(start_page).unwrap().ppn();
            return Some(&mut ppn.read_as_bytes_array()[slice_begin..slice_end]);
        } else {
            return None;
        }
    }
}

pub struct PageAlignedVirtBufIter {
    range: VPNRange,
    page_table_entry: TopLevelEntry,
}

impl Reader<&[u8]> for PageAlignedVirtBufIter {
    fn read(&mut self, src: &[u8]) -> usize {
        let mut written = 0;
        for vpn in self.range {
            let len = core::cmp::min(PAGE_SIZE, src.len() - written);
            self.page_table_entry
                .translate(vpn)
                .unwrap()
                .ppn()
                .read_as_bytes_array()[..len]
                .copy_from_slice(&src[written..written + len]);
            written += len;
        }
        written
    }
}

impl Reader<PageAlignedVirtBufIter> for PageAlignedVirtBufIter {
    fn read(&mut self, src: PageAlignedVirtBufIter) -> usize {
        let mut written = 0;
        let dst_page_table_entry = self.page_table_entry;
        let src_page_table_entry = src.page_table_entry;
        for (i, j) in self.range.into_iter().zip(src.range.into_iter()) {
            let dst_ppn = dst_page_table_entry.translate(i).unwrap().ppn();
            let src_ppn = src.page_table_entry.translate(j).unwrap().ppn();
            *dst_ppn.read_as_bytes_array() = *src_ppn.read_as_bytes_array();
            written += PAGE_SIZE;
        }
        written
    }
}

impl PageAlignedVirtBufIter {
    pub fn new(range: VPNRange, page_table_entry: TopLevelEntry) -> Self {
        Self {
            range,
            page_table_entry,
        }
    }
}
