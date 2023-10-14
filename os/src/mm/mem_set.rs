use core::{arch::asm, ops::Range};

use alloc::vec::Vec;
use riscv::register::satp;
use xmas_elf::ElfFile;

use crate::{
    constant::{MEM_END_PPN, TRAMPOLINE_VPN, TRAP_CONTEXT_VPN, USER_STACK_SIZE_BY_PAGE},
    mm::address::VirtAddr,
    sync::up::UPSafeCell,
};

use super::{
    address::{PhysPageNum, VirtPageNum},
    page_table::{PTEFlags, PageTableEntry, TopLevelEntry},
    virt_mem_area::{MapType, Permission, VirtMemArea},
};

//进程内存描述符
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
    pub fn insert_framed_area(&mut self, range: Range<VirtPageNum>, perm: Permission) {
        self.push_vma(VirtMemArea::new(range, MapType::Framed, perm))
    }

    fn insert_identical_area(&mut self, range: Range<PhysPageNum>, perm: Permission) {
        self.push_vma(VirtMemArea::new(
            range.start.identical_map()..range.end.identical_map(),
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
        let text_seg = stext()..etext();
        let rodata_seg = srodata()..erodata();
        let data_seg = sdata()..edata();
        let bss_seg = sbss_with_stack()..ebss();
        let phys_mem = ekernel()..MEM_END_PPN;
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
                let vma = VirtMemArea::new(start_va.floor()..end_va.ceil(), MapType::Framed, perm);
                max_end_vpn = vma.end();
                mem_set.push_vma_with_data(
                    vma,
                    &elf.input[ph.offset() as usize..][..ph.file_size() as usize],
                );
            }
        }
        let stack_top = max_end_vpn + 1; //空出一个页, 越界时就能触发页异常
        let stack_bottom = stack_top + USER_STACK_SIZE_BY_PAGE;
        //用户栈
        mem_set.insert_framed_area(
            stack_top..stack_bottom,
            Permission::R | Permission::W | Permission::U,
        );
        //保存中断上下文的内存区域
        mem_set.insert_framed_area(
            TRAP_CONTEXT_VPN..TRAMPOLINE_VPN,
            Permission::R | Permission::W,
        );
        (
            mem_set,
            stack_bottom,
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
}

lazy_static! {
    pub static ref KERNEL_MEM_SPACE: UPSafeCell<MemSet> =
        unsafe { UPSafeCell::new(MemSet::new_kernel()) };
}
