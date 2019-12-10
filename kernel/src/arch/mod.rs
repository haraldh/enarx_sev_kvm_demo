#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[cfg(target_arch = "x86_64")]
pub mod prelude {
    pub use super::x86_64::exec_app;
    pub use super::x86_64::init;
    pub use super::x86_64::serial;
}
