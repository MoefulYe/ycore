use std::sync::Arc;

use yfs::vfs::Vnode;

pub const MAX_FDT_SIZE: usize = 8;

pub struct FdtEntryInner {
    pub vnode: Arc<Vnode>,
    pub readable: bool,
    pub writable: bool,
    pub offset: u32,
}
pub struct FdtEntry(Option<FdtEntryInner>);
type FdTable = [FdtEntry; MAX_FDT_SIZE];
