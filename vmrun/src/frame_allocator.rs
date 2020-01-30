use vmbootspec::{MemoryMap, MemoryRegion, MemoryRegionType};

pub(crate) struct FrameAllocator {
    pub memory_map: MemoryMap,
}

impl FrameAllocator {
    /// Marks the passed region in the memory map.
    ///
    /// Panics if a non-usable region (e.g. a reserved region) overlaps with the passed region.
    pub(crate) fn mark_allocated_region(&mut self, region: MemoryRegion) {
        let mut region = region.clone();
        for r in self.memory_map.iter_mut() {
            // New region inside region of same type
            if r.region_type == region.region_type
                && r.range.start_frame_number <= region.range.start_frame_number
                && r.range.end_frame_number >= region.range.end_frame_number
            {
                return;
            }

            // New region extends old region
            if r.region_type == region.region_type
                && r.range.start_frame_number <= region.range.start_frame_number
                && r.range.end_frame_number > region.range.start_frame_number
                && r.range.end_frame_number <= region.range.end_frame_number
            {
                region.range.start_frame_number = r.range.end_frame_number;
            }

            if region.range.start_frame_number >= r.range.end_frame_number {
                continue;
            }
            if region.range.end_frame_number <= r.range.start_frame_number {
                continue;
            }

            if r.region_type != MemoryRegionType::Usable {
                panic!(
                    "region {:x?} overlaps with non-usable region {:x?}",
                    region, r
                );
            }

            if region.range.start_frame_number == r.range.start_frame_number {
                if region.range.end_frame_number < r.range.end_frame_number {
                    // Case: (r = `r`, R = `region`)
                    // ----rrrrrrrrrrr----
                    // ----RRRR-----------
                    r.range.start_frame_number = region.range.end_frame_number;
                    self.memory_map.add_region(region);
                } else {
                    // Case: (r = `r`, R = `region`)
                    // ----rrrrrrrrrrr----
                    // ----RRRRRRRRRRRRRR-
                    *r = region;
                }
            } else if region.range.start_frame_number > r.range.start_frame_number {
                if region.range.end_frame_number < r.range.end_frame_number {
                    // Case: (r = `r`, R = `region`)
                    // ----rrrrrrrrrrr----
                    // ------RRRR---------
                    let mut behind_r = *r;
                    behind_r.range.start_frame_number = region.range.end_frame_number;
                    r.range.end_frame_number = region.range.start_frame_number;
                    self.memory_map.add_region(behind_r);
                    self.memory_map.add_region(region);
                } else {
                    // Case: (r = `r`, R = `region`)
                    // ----rrrrrrrrrrr----
                    // -----------RRRR---- or
                    // -------------RRRR--
                    r.range.end_frame_number = region.range.start_frame_number;
                    self.memory_map.add_region(region);
                }
            } else {
                // Case: (r = `r`, R = `region`)
                // ----rrrrrrrrrrr----
                // --RRRR-------------
                r.range.start_frame_number = region.range.end_frame_number;
                self.memory_map.add_region(region);
            }
            return;
        }
        panic!(
            "region {:x?} is not a usable memory region\n{:#?}",
            region, self.memory_map
        );
    }
}
