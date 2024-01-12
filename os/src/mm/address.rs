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

impl Display for PhysAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

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

impl Add<isize> for PhysAddr {
    type Output = PhysAddr;

    fn add(self, rhs: isize) -> Self::Output {
        Self(self.0 + rhs as usize)
    }
}

impl Sub<isize> for PhysAddr {
    type Output = PhysAddr;

    fn sub(self, rhs: isize) -> Self::Output {
        Self(self.0 - rhs as usize)
    }
}

impl Sub for PhysAddr {
    type Output = isize;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0 as isize - rhs.0 as isize
    }
}

impl From<usize> for PhysAddr {
    fn from(v: usize) -> Self {
        Self(v & PA_MASK)
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

    pub fn raw(self) -> usize {
        self.0
    }

    pub fn identical(self) -> VirtAddr {
        VirtAddr(self.0)
    }
}

//39位 符号拓展
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(pub usize);

impl Display for VirtAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

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

impl Add<isize> for VirtAddr {
    type Output = VirtAddr;

    fn add(self, rhs: isize) -> Self::Output {
        Self(self.0 + rhs as usize)
    }
}

impl Sub<isize> for VirtAddr {
    type Output = VirtAddr;

    fn sub(self, rhs: isize) -> Self::Output {
        Self(self.0 - rhs as usize)
    }
}

impl Sub<VirtAddr> for VirtAddr {
    type Output = isize;

    fn sub(self, rhs: VirtAddr) -> Self::Output {
        self.0 as isize - rhs.0 as isize
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

    pub fn identical(self) -> PhysAddr {
        PhysAddr(self.0)
    }

    /// 顶层页表对应的索引号
    pub fn vpn0(self) -> usize {
        self.0 >> 30 & 0x1ff
    }

    /// 二级页表对应的索引号
    pub fn vpn1(self) -> usize {
        self.0 >> 21 & 0x1ff
    }

    /// 三级页表对应的索引号
    pub fn vpn2(self) -> usize {
        self.0 >> 12 & 0x1ff
    }

    pub fn vpns(self) -> [usize; 3] {
        [self.vpn0(), self.vpn1(), self.vpn2()]
    }

    pub fn raw(self) -> usize {
        self.0
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

impl AddAssign<isize> for PhysPageNum {
    fn add_assign(&mut self, rhs: isize) {
        self.0 += rhs as usize;
    }
}

impl SubAssign<isize> for PhysPageNum {
    fn sub_assign(&mut self, rhs: isize) {
        self.0 -= rhs as usize;
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

impl Add<isize> for PhysPageNum {
    type Output = PhysPageNum;

    fn add(self, rhs: isize) -> Self::Output {
        Self(self.0 + rhs as usize)
    }
}

impl Sub<isize> for PhysPageNum {
    type Output = PhysPageNum;

    fn sub(self, rhs: isize) -> Self::Output {
        Self(self.0 - rhs as usize)
    }
}

impl Sub for PhysPageNum {
    type Output = isize;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0 as isize - rhs.0 as isize
    }
}

impl From<usize> for PhysPageNum {
    fn from(v: usize) -> Self {
        Self(v & PPN_MASK)
    }
}

impl PhysPageNum {
    pub const NULL: PhysPageNum = PhysPageNum(0);

    pub fn identical(self) -> VirtPageNum {
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

    pub fn raw(self) -> usize {
        self.0
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

impl From<usize> for VirtPageNum {
    fn from(v: usize) -> Self {
        Self(v & VPN_MASK)
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

impl Add<isize> for VirtPageNum {
    type Output = VirtPageNum;

    fn add(self, rhs: isize) -> Self::Output {
        Self(self.0 + rhs as usize)
    }
}

impl Sub<isize> for VirtPageNum {
    type Output = VirtPageNum;

    fn sub(self, rhs: isize) -> Self::Output {
        Self(self.0 - rhs as usize)
    }
}

impl Sub for VirtPageNum {
    type Output = isize;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0 as isize - rhs.0 as isize
    }
}

impl VirtPageNum {
    pub const NULL: VirtPageNum = VirtPageNum(0);
    pub fn identical(self) -> PhysPageNum {
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

    pub fn raw(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct VirtPageSpan {
    pub start: VirtPageNum,
    pub end: VirtPageNum,
}

impl VirtPageSpan {
    pub fn identical(self) -> PhysPageSpan {
        PhysPageSpan {
            start: self.start.identical(),
            end: self.end.identical(),
        }
    }
}

impl VirtPageSpan {
    pub fn new(range: Range<VirtPageNum>) -> Self {
        range.into()
    }

    pub fn size(&self) -> usize {
        (self.end.raw() - self.start.raw()) << PAGE_SIZE_BITS
    }
}

impl Iterator for VirtPageSpan {
    type Item = VirtPageNum;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            None
        } else {
            let ret = self.start;
            self.start = ret + 1usize;
            Some(ret)
        }
    }
}

impl From<Range<VirtPageNum>> for VirtPageSpan {
    fn from(Range { start, end }: Range<VirtPageNum>) -> Self {
        Self { start, end }
    }
}

#[derive(Clone, Copy)]
pub struct PhysPageSpan {
    pub start: PhysPageNum,
    pub end: PhysPageNum,
}

impl PhysPageSpan {
    pub fn identical(self) -> VirtPageSpan {
        VirtPageSpan {
            start: self.start.identical(),
            end: self.end.identical(),
        }
    }
}

impl Iterator for PhysPageSpan {
    type Item = PhysPageNum;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            None
        } else {
            let ret = self.start;
            self.start = ret + 1usize;
            Some(ret)
        }
    }
}

impl From<Range<PhysPageNum>> for PhysPageSpan {
    fn from(Range { start, end }: Range<PhysPageNum>) -> Self {
        Self { start, end }
    }
}

#[derive(Clone, Copy)]
pub struct VirtAddrSpan {
    pub start: VirtAddr,
    pub end: VirtAddr,
}

impl From<Range<VirtAddr>> for VirtAddrSpan {
    fn from(Range { start, end }: Range<VirtAddr>) -> Self {
        Self { start, end }
    }
}

impl Iterator for VirtAddrSpan {
    type Item = VirtAddr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            None
        } else {
            let ret = self.start;
            self.start = ret + 1usize;
            Some(ret)
        }
    }
}

impl VirtAddrSpan {
    pub fn identical(self) -> PhysAddrSpan {
        PhysAddrSpan {
            start: self.start.identical(),
            end: self.end.identical(),
        }
    }
}

#[derive(Copy, Clone)]
pub struct PhysAddrSpan {
    pub start: PhysAddr,
    pub end: PhysAddr,
}

impl PhysAddrSpan {
    fn identical(self) -> VirtAddrSpan {
        VirtAddrSpan {
            start: self.start.identical(),
            end: self.end.identical(),
        }
    }
}

impl Iterator for PhysAddrSpan {
    type Item = PhysAddr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            None
        } else {
            let ret = self.start;
            self.start = ret + 1usize;
            Some(ret)
        }
    }
}

impl From<Range<PhysAddr>> for PhysAddrSpan {
    fn from(Range { start, end }: Range<PhysAddr>) -> Self {
        Self { start, end }
    }
}

pub trait Reader<T> {
    fn read(&mut self, src: T) -> usize {
        0
    }
}

pub struct UserBuffer {
    span: VirtAddrSpan,
    page_table_entry: TopLevelEntry,
}

impl UserBuffer {
    pub fn new(span: impl Into<VirtAddrSpan>, page_table_entry: TopLevelEntry) -> Self {
        Self {
            span: span.into(),
            page_table_entry,
        }
    }
}

/// 对用户空间的缓冲区按页边界进行切割, 返回每一个页内的切片
impl Iterator for UserBuffer {
    type Item = &'static mut [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.span.start < self.span.end {
            let (start_page, start_offset) = self.span.start.split();
            let (end_page, end_offset) = self.span.end.split();
            self.span.start = start_page.ceil();
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

impl Reader<&[u8]> for UserBuffer {
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

pub struct PageAlignedVirtBufIter {
    range: VirtPageSpan,
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
            dst_ppn
                .read_as_bytes_array()
                .copy_from_slice(src_ppn.read_as_bytes_array());
            written += PAGE_SIZE;
        }
        written
    }
}

impl PageAlignedVirtBufIter {
    pub fn new(range: VirtPageSpan, page_table_entry: TopLevelEntry) -> Self {
        Self {
            range,
            page_table_entry,
        }
    }
}
