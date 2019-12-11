//! Global Descriptor Table
//!
//! Copied from the x86_64 crate with one more entry

use x86_64::structures::gdt::{Descriptor, DescriptorFlags, SegmentSelector};
use x86_64::PrivilegeLevel;

/// A 64-bit mode global descriptor table (GDT).
///
/// In 64-bit mode, segmentation is not supported. The GDT is used nonetheless, for example for
/// switching between user and kernel mode or for loading a TSS.
///
/// The GDT has a fixed size of 9 entries, trying to add more entries will panic.
///
/// You do **not** need to add a null segment descriptor yourself - this is already done
/// internally.
///
/// Data segment registers in ring 0 can be loaded with the null segment selector. When running in
/// ring 3, the `ss` register must point to a valid data segment which can be obtained through the
/// [`Descriptor::user_data_segment()`](Descriptor::user_data_segment) function. Code segments must
/// be valid and non-null at all times and can be obtained through the
/// [`Descriptor::kernel_code_segment()`](Descriptor::kernel_code_segment) and
/// [`Descriptor::user_code_segment()`](Descriptor::user_code_segment) in rings 0 and 3
/// respectively.
///
/// For more info, see:
/// [x86 Instruction Reference for `mov`](https://www.felixcloutier.com/x86/mov#64-bit-mode-exceptions),
/// [Intel Manual](https://software.intel.com/sites/default/files/managed/39/c5/325462-sdm-vol-1-2abcd-3abcd.pdf),
/// [AMD Manual](https://www.amd.com/system/files/TechDocs/24593.pdf)
///
/// # Example
/// ```
/// use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor};
///
/// let mut gdt = GlobalDescriptorTable::new();
/// gdt.add_entry(Descriptor::kernel_code_segment());
/// gdt.add_entry(Descriptor::user_code_segment());
/// gdt.add_entry(Descriptor::user_data_segment());
///
/// // Add entry for TSS, call gdt.load() then update segment registers
/// ```

#[derive(Debug, Clone)]
pub struct GlobalDescriptorTable {
    table: [u64; 9],
    next_free: usize,
}

impl GlobalDescriptorTable {
    /// Creates an empty GDT.
    pub fn new() -> GlobalDescriptorTable {
        GlobalDescriptorTable {
            table: [0; 9],
            next_free: 1,
        }
    }

    /// Adds the given segment descriptor to the GDT, returning the segment selector.
    ///
    /// Panics if the GDT has no free entries left.
    pub fn add_entry(&mut self, entry: Descriptor) -> SegmentSelector {
        let (index, rpl) = match entry {
            Descriptor::UserSegment(value) => (
                self.push(value),
                if DescriptorFlags::from_bits_truncate(value).contains(DescriptorFlags::DPL_RING_3)
                {
                    PrivilegeLevel::Ring3
                } else {
                    PrivilegeLevel::Ring0
                },
            ),
            Descriptor::SystemSegment(value_low, value_high) => {
                let index = self.push(value_low);
                self.push(value_high);
                (index, PrivilegeLevel::Ring0)
            }
        };
        SegmentSelector::new(index as u16, rpl)
    }

    /// Loads the GDT in the CPU using the `lgdt` instruction. This does **not** alter any of the
    /// segment registers; you **must** (re)load them yourself using [the appropriate
    /// functions](crate::instructions::segmentation):
    /// [load_ss](crate::instructions::segmentation::load_ss),
    /// [set_cs](crate::instructions::segmentation::set_cs).
    #[cfg(target_arch = "x86_64")]
    pub fn load(&'static self) {
        use core::mem::size_of;
        use x86_64::instructions::tables::{lgdt, DescriptorTablePointer};

        let ptr = DescriptorTablePointer {
            base: self.table.as_ptr() as u64,
            limit: (self.table.len() * size_of::<u64>() - 1) as u16,
        };

        unsafe { lgdt(&ptr) };
    }

    fn push(&mut self, value: u64) -> usize {
        if self.next_free < self.table.len() {
            let index = self.next_free;
            self.table[index] = value;
            self.next_free += 1;
            index
        } else {
            panic!("GDT full");
        }
    }
}
