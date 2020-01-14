//! Duplicated from the x86_64 crate, because of some modifications
//!
//! can be removed, if we can get patches upstream
//! see https://github.com/rust-osdev/x86_64/pull/114

// Duplicated, because we want USER_ACCESSIBLE in newly allocated page tables
pub mod paging;
pub use paging::OffsetPageTable;
