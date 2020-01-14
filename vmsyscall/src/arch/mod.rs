pub mod x86_64_syscall;
pub mod x86_errno;

#[cfg(target_arch = "x86_64")]
pub use x86_errno as errno;

#[cfg(target_arch = "x86_64")]
pub use x86_64_syscall as syscall;
