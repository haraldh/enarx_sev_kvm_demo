//! Duplicated from the x86_64 crate, because of some modifications
//!
//! might be removed, if we can get patches upstream

// Duplicated, because we want USER_ACCESSIBLE in newly allocated page tables
pub mod paging;
pub use paging::OffsetPageTable;

// Because we need one more entry in the table
pub mod gdt;
