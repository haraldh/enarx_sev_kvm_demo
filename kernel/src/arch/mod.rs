#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[cfg(target_arch = "x86_64")]
pub use self::x86_64::{exec_elf, init, serial, structures::paging::OffsetPageTable};
