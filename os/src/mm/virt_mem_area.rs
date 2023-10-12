use core::ops::Range;

use alloc::collections::BTreeMap;

use crate::constant::PAGE_SIZE;

use super::{
    address::{PhysPageNum, VPNRange, VirtPageNum},
    frame_alloc::ALLOCATOR,
    page_table::{PTEFlags, TopLevelEntry},
};

enum Map {
    Identical,
    Framed(BTreeMap<VirtPageNum, PhysPageNum>),
}

impl Map {
    fn is_framed(&self) -> bool {
        match self {
            Map::Identical => false,
            Map::Framed(_) => true,
        }
    }
}

pub enum MapType {
    Identical,
    Framed,
}

bitflags! {
    pub struct Permission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

pub struct VirtMemArea {
    vpn_range: VPNRange,
    map: Map,
    perm: Permission,
}

impl VirtMemArea {
    pub fn new(vpn_range: Range<VirtPageNum>, map_type: MapType, perm: Permission) -> Self {
        let map = match map_type {
            MapType::Identical => Map::Identical,
            MapType::Framed => Map::Framed(BTreeMap::new()),
        };
        Self {
            vpn_range: vpn_range.into(),
            map,
            perm,
        }
    }

    //传入一个顶层页表基址和一个虚拟页号, 让帧分配器分配一个物理页帧, 分别在页表和vma中建立映射关系
    pub fn map_one(&mut self, page_table_entry: TopLevelEntry, vpn: VirtPageNum) {
        let ppn = match self.map {
            Map::Identical => PhysPageNum(vpn.0),
            Map::Framed(ref mut map) => {
                let ppn = ALLOCATOR.exclusive_access().alloc();
                map.insert(vpn, ppn);
                ppn
            }
        };
        let pte_flags = self.perm.into();
        page_table_entry.map(vpn, ppn, pte_flags);
    }

    //传入一个顶层页表基址和一个被映射的虚拟页号, 从页表和vma中删除映射关系,
    //page_table_entry::unmap方法内部会调用dealloc方法回收物理页帧
    pub fn unmap_one(&mut self, page_table_entry: TopLevelEntry, vpn: VirtPageNum) {
        if let Map::Framed(ref mut map) = self.map {
            map.remove(&vpn);
        }
        page_table_entry.unmap(vpn);
    }

    pub fn map(&mut self, page_table_entry: TopLevelEntry) {
        for vpn in self.vpn_range {
            self.map_one(page_table_entry, vpn)
        }
    }

    pub fn unmap(&mut self, page_table_entry: TopLevelEntry) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table_entry, vpn)
        }
    }

    pub fn shrink_to(&mut self, page_table_entry: TopLevelEntry, new_end: VirtPageNum) {
        assert!(
            new_end >= self.vpn_range.start && new_end <= self.vpn_range.end,
            "new_end must be in the range of vma"
        );
        for vpn in VPNRange::new(new_end..self.vpn_range.end) {
            self.unmap_one(page_table_entry, vpn)
        }
        self.vpn_range.end = new_end;
    }

    pub fn append_to(&mut self, page_table_entry: TopLevelEntry, new_end: VirtPageNum) {
        assert!(
            new_end >= self.vpn_range.end,
            "new_end must be greater than the end of vma"
        );
        for vpn in VPNRange::new(self.vpn_range.end..new_end) {
            self.map_one(page_table_entry, vpn)
        }
        self.vpn_range.end = new_end;
    }

    pub fn memcpy(&mut self, page_table_entry: TopLevelEntry, src: &[u8]) {
        assert!(self.map.is_framed(), "vma must be framed");
        let mut start = 0usize;
        let mut cur_vpn = self.vpn_range.start;
        let len = src.len();
        assert!(len < self.vpn_range.size(), "data is too large");
        loop {
            let this_cpy_src = &src[start..len.min(start + PAGE_SIZE)];
            let this_cpy_dst = &mut page_table_entry
                .translate(cur_vpn)
                .unwrap()
                .ppn()
                .read_as_bytes_array()
                .as_mut_slice()[..this_cpy_src.len()];
            this_cpy_dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            cur_vpn += 1;
        }
    }
}
