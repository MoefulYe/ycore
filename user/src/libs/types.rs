pub type CStr = *const u8;
pub type Fd = usize;
pub type Ms = usize;
pub type Pid = usize;
pub type ExitCode = i32;
pub type Argv = [&'static str];
pub type Result<T = (), E = ()> = core::result::Result<T, E>;
