#![allow(unused)]
use core::{arch::asm, ops::Range};

use alloc::vec::Vec;
use log::{debug, info};
use riscv::register::satp;
use xmas_elf::ElfFile;

use crate::{
    constant::{MEM_END_PPN, TRAMPOLINE_VPN, TRAP_CONTEXT_VPN, USER_STACK_SIZE_BY_PAGE},
    mm::address::VirtAddr,
    sync::up::UPSafeCell,
};

use super::{
    address::{
        PageAlignedVirtBufIter, PhysPageNum, PhysPageSpan, Reader, VirtPageNum, VirtPageSpan,
    },
    page_table::{PTEFlags, PageTableEntry, TopLevelEntry},
    virt_mem_area::{MapType, Permission, VirtMemArea},
};

//进程内存描述符
pub struct MemSet {
    entry: TopLevelEntry,
    vmas: Vec<VirtMemArea>,
    heap_start: VirtPageNum,
}

impl Clone for MemSet {
    fn clone(&self) -> Self {
        let mut mem_set = Self::new_bare();
        mem_set.map_trampoline();
        mem_set.heap_start = self.heap_start;
        for vma in &self.vmas {
            //克隆一个新vma包括range和perm等信息, 但是还没有建立vpn到ppn的映射关系,
            //因为新的内存空间会映射到不同的ppn上旧的ppn对于新内存空间是没有意义的所以映射关系要等下自己创建
            let new = vma.clone();
            //把新的vma加入到内存描述符中, 会建立vpn到ppn的映射关系
            mem_set.push_vma(new);
            let mut iter_dst = PageAlignedVirtBufIter::new(vma.range(), mem_set.entry);
            let iter_src = PageAlignedVirtBufIter::new(vma.range(), self.entry);
            //拷贝数据
            iter_dst.read(iter_src);
        }
        mem_set
    }
}

impl MemSet {
    pub fn new_bare() -> Self {
        Self {
            entry: TopLevelEntry::new(),
            vmas: Vec::new(),
            heap_start: VirtPageNum::NULL,
        }
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.entry.translate(vpn)
    }

    //创建一个逻辑上的虚拟内存段后(对于framed区域来说此时虚拟页还没有映射到物理内存页上,在操作后会建立映射关系) 把虚拟内存段挂载到MemSet上
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
    pub fn insert_framed_area(&mut self, range: VirtPageSpan, perm: Permission) {
        self.push_vma(VirtMemArea::new(range, MapType::Framed, perm))
    }

    fn insert_identical_area(&mut self, range: PhysPageSpan, perm: Permission) {
        self.push_vma(VirtMemArea::new(
            range.identical(),
            MapType::Identical,
            perm,
        ))
    }

    fn map_trampoline(&mut self) {
        self.entry.map(
            TRAMPOLINE_VPN,
            super::kernel_layout::strampoline(),
            PTEFlags::READ | PTEFlags::EXEC,
        )
    }

    pub fn new_kernel() -> Self {
        use super::kernel_layout::*;
        let mut mem_set = Self::new_bare();
        let text_seg: PhysPageSpan = (stext()..etext()).into();
        let rodata_seg: PhysPageSpan = (srodata()..erodata()).into();
        let data_seg: PhysPageSpan = (sdata()..edata()).into();
        let bss_seg: PhysPageSpan = (sbss_with_stack()..ebss()).into();
        let phys_mem: PhysPageSpan = (ekernel()..MEM_END_PPN).into();
        info!(
            "[kenrel-memory-space] .text [{},{})",
            text_seg.start, text_seg.end
        );
        info!(
            "[kenrel-memory-space] .rodata [{},{})",
            rodata_seg.start, rodata_seg.end
        );
        info!(
            "[kenrel-memory-space] .data [{},{})",
            data_seg.start, data_seg.end
        );
        info!(
            "[kenrel-memory-space] .bss [{},{})",
            bss_seg.start, bss_seg.end
        );
        info!(
            "[kenrel-memory-space] physical memory [{},{})",
            phys_mem.start, phys_mem.end
        );
        mem_set.map_trampoline();
        mem_set.insert_identical_area(text_seg, Permission::R | Permission::X);
        mem_set.insert_identical_area(rodata_seg, Permission::R);
        mem_set.insert_identical_area(data_seg, Permission::R | Permission::W);
        mem_set.insert_identical_area(bss_seg, Permission::R | Permission::W);
        mem_set.insert_identical_area(phys_mem, Permission::R | Permission::W);
        mem_set
    }

    //内存描述符, 用户栈底, 程序入口地址
    pub fn from_elf(elf_data: &[u8]) -> (Self, VirtPageNum, VirtAddr) {
        let mut mem_set = Self::new_bare();
        //最高地址映射到跳板代码
        mem_set.map_trampoline();
        let elf = ElfFile::new(elf_data).unwrap();
        let header = elf.header;
        assert_eq!(header.pt1.magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf");
        let ph_count = header.pt2.ph_count();
        let mut max_end_vpn = VirtPageNum::NULL;
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if let xmas_elf::program::Type::Load = ph.get_type().unwrap() {
                let start_va: VirtAddr = ph.virtual_addr().into();
                let end_va: VirtAddr = (ph.virtual_addr() + ph.mem_size()).into();
                let mut perm = Permission::U;
                let flags = ph.flags();
                if flags.is_read() {
                    perm |= Permission::R;
                }
                if flags.is_write() {
                    perm |= Permission::W;
                }
                if flags.is_execute() {
                    perm |= Permission::X;
                }
                let vma = VirtMemArea::new(
                    (start_va.floor()..end_va.ceil()).into(),
                    MapType::Framed,
                    perm,
                );
                max_end_vpn = vma.end();
                mem_set.push_vma_with_data(
                    vma,
                    &elf.input[ph.offset() as usize..][..ph.file_size() as usize],
                );
            }
        }
        let user_stack_top = max_end_vpn + 1usize; //空出一个页, 越界时就能触发页异常
        let user_stack_bottom = user_stack_top + USER_STACK_SIZE_BY_PAGE;
        //用户栈
        mem_set.insert_framed_area(
            (user_stack_top..user_stack_bottom).into(),
            Permission::R | Permission::W | Permission::U,
        );
        //堆空间
        mem_set.insert_framed_area(
            (user_stack_bottom..user_stack_bottom).into(),
            Permission::R | Permission::W | Permission::U,
        );
        mem_set.heap_start = user_stack_bottom;
        //保存中断上下文的内存区域
        mem_set.insert_framed_area(
            (TRAP_CONTEXT_VPN..TRAMPOLINE_VPN).into(),
            Permission::R | Permission::W,
        );
        (
            mem_set,
            user_stack_bottom,
            (elf.header.pt2.entry_point() as usize).into(),
        )
    }

    pub fn token(&self) -> usize {
        self.entry.token()
    }

    pub fn activate(&self) {
        let satp = self.entry.token();
        unsafe {
            satp::write(satp);
            asm!("sfence.vma");
        }
    }

    pub fn recycle(&mut self) {
        for vma in &mut self.vmas {
            vma.unmap(self.entry);
        }
        self.entry.drop();
    }

    pub fn heap_grow(&mut self, new_end: VirtPageNum) {
        self.vmas
            .iter_mut()
            .find(|vma| vma.start() == self.heap_start)
            .unwrap()
            .append_to(self.entry, new_end)
    }

    pub fn heap_shrink(&mut self, new_end: VirtPageNum) {
        self.vmas
            .iter_mut()
            .find(|vma| vma.start() == self.heap_start)
            .unwrap()
            .shrink_to(self.entry, new_end)
    }
}

lazy_static! {
    pub static ref KERNEL_MEM_SPACE: UPSafeCell<MemSet> = unsafe {
        info!("[kernel] init kernel memory space");
        UPSafeCell::new(MemSet::new_kernel())
    };
}
