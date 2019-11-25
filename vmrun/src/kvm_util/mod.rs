use bitfield::Bitfield;
use kvm_bindings::{kvm_dtable, kvm_mp_state, kvm_segment, kvm_userspace_memory_region};
use kvm_ioctls::{Kvm, VcpuFd, VmFd, MAX_KVM_CPUID_ENTRIES};
use nix;
pub use x86_64::{HostVirtAddr, PhysAddr, VirtAddr};
mod bitfield;
mod frame_allocator;
mod gdt;
mod page_table;
mod x86;
mod x86_64;

use crate::error::*;
use crate::kvm_util::bitfield::Rangefield;
use crate::map_context;
use bootinfo::{BootInfo, FrameRange, MemoryMap, MemoryRegion, MemoryRegionType};

use crate::kvm_util::x86_64::structures::paging::Size4KiB;
use core::ptr::null_mut;
use nix::sys::mman::{madvise, mmap, MapFlags, ProtFlags};
use page_table::PageTableFlags;
use x86::*;
use x86_64::structures::paging::{frame::PhysFrameRange, PhysFrame};

const KVM_UTIL_PGS_PER_HUGEPG: usize = 512;
const DEFAULT_GUEST_PHY_PAGES: u64 = 512;
const KVM_GUEST_PAGE_TABLE_MIN_PADDR: u64 = 0x18_0000;
const KVM_UTIL_MIN_VADDR: u64 = 0x2000;
const KVM_UTIL_MIN_PFN: u64 = 2;
const DEFAULT_STACK_PGS: u64 = 5;
const DEFAULT_GUEST_STACK_VADDR_MIN: u64 = 0xab_6000;
const PHYSICAL_MEMORY_OFFSET: u64 = 0x0000_7000_0000_0000;

struct UserspaceMemRegion {
    region: kvm_userspace_memory_region,
    used_phy_pages: Bitfield,
    host_mem: PhysAddr,
    mmap_start: PhysAddr,
    mmap_size: usize,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub enum VmMemBackingSrcType {
    VM_MEM_SRC_ANONYMOUS,
    VM_MEM_SRC_ANONYMOUS_THP,
    VM_MEM_SRC_ANONYMOUS_HUGETLB,
}

pub struct KvmVm {
    kvm: Kvm,
    pub cpu_fd: Vec<VcpuFd>,
    kvm_fd: VmFd,
    page_size: u64,
    page_shift: u32,
    pa_bits: u32,
    va_bits: u32,
    max_gfn: u64,
    frame_allocator: frame_allocator::FrameAllocator,
    userspace_mem_regions: Vec<UserspaceMemRegion>,
    vpages_valid: Rangefield,
    vpages_mapped: Bitfield,
    has_irqchip: bool,
    pgd: Option<PhysAddr>,
    gdt: Option<VirtAddr>,
    tss: Option<VirtAddr>,
}

fn frame_range(range: PhysFrameRange) -> FrameRange {
    FrameRange::new(
        range.start.start_address().as_u64(),
        range.end.start_address().as_u64(),
    )
}

fn phys_frame_range(range: FrameRange) -> PhysFrameRange {
    PhysFrameRange {
        start: PhysFrame::from_start_address(PhysAddr::new(range.start_addr())).unwrap(),
        end: PhysFrame::from_start_address(PhysAddr::new(range.end_addr())).unwrap(),
    }
}

impl KvmVm {
    pub fn vm_create(phy_pages: u64) -> Result<Self, Error> {
        let kvm = Kvm::new().unwrap();

        let kvm_fd: VmFd = kvm.create_vm().map_err(map_context!())?;

        let mut vm = KvmVm {
            kvm,
            cpu_fd: vec![],
            kvm_fd,
            page_size: 0x1000,
            page_shift: 12,
            pa_bits: 52,
            va_bits: 48,
            max_gfn: 0,
            frame_allocator: frame_allocator::FrameAllocator {
                memory_map: MemoryMap::new(),
            },
            userspace_mem_regions: vec![],
            vpages_valid: Default::default(),
            vpages_mapped: Default::default(),
            has_irqchip: false,
            pgd: None,
            gdt: None,
            tss: None,
        };

        let end = ((1u64 << (vm.va_bits - 1)) >> vm.page_shift) as _;
        vm.vpages_valid.push(0..end);
        /*
                for i in 0..((1usize << (vm.va_bits as usize - 1)) >> vm.page_shift as usize) {
                    vm.vpages_valid.set(i, true);
                }
        */
        let start = (!((1u64 << (vm.va_bits - 1)) - 1)) >> vm.page_shift;
        let len = (1u64 << (vm.va_bits - 1)) >> vm.page_shift;

        vm.vpages_valid.push(start as _..(start + len) as _);
        /*
                for i in start..(start + len) {
                    vm.vpages_valid.set(i, true);
                }
        */
        vm.max_gfn = ((1u64 << vm.pa_bits) >> vm.page_shift) - 1;

        if phy_pages != 0 {
            vm.vm_userspace_mem_region_add(
                VmMemBackingSrcType::VM_MEM_SRC_ANONYMOUS,
                PhysAddr::new(0),
                0,
                phy_pages,
                0,
            )?;
        }

        Ok(vm)
    }

    pub fn vm_userspace_mem_region_add(
        &mut self,
        src_type: VmMemBackingSrcType,
        guest_paddr: PhysAddr,
        slot: u32,
        npages: u64,
        flags: u32,
    ) -> Result<(), Error> {
        let huge_page_size: usize = KVM_UTIL_PGS_PER_HUGEPG * self.page_size as usize;

        if (guest_paddr.as_u64() % self.page_size) != 0 {
            return Err(ErrorKind::Generic.into()); // FIXME: Error
        }

        if (guest_paddr.as_u64() >> self.page_shift as u64) + npages - 1 > self.max_gfn {
            return Err(ErrorKind::Generic.into()); // FIXME: Error
        }

        for r in self.userspace_mem_regions.iter() {
            if r.region.slot == slot {
                return Err(ErrorKind::MemRegionWithSlotAlreadyExists.into());
            }

            if guest_paddr.as_u64() <= (r.region.guest_phys_addr + r.region.memory_size)
                && (guest_paddr.as_u64() + npages * self.page_size) >= r.region.guest_phys_addr
            {
                return Err(ErrorKind::OverlappingUserspaceMemRegionExists.into());
            }
        }

        let mut region = UserspaceMemRegion {
            region: Default::default(),
            used_phy_pages: Default::default(),
            host_mem: PhysAddr::new(0),
            mmap_start: PhysAddr::new(0),
            mmap_size: (npages * self.page_size) as _,
        };

        if let VmMemBackingSrcType::VM_MEM_SRC_ANONYMOUS_THP = src_type {
            region.mmap_size += huge_page_size;
        }

        let mmap_start = unsafe {
            mmap(
                null_mut(),
                region.mmap_size,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_PRIVATE
                    | MapFlags::MAP_ANONYMOUS
                    | match src_type {
                        VmMemBackingSrcType::VM_MEM_SRC_ANONYMOUS_HUGETLB => MapFlags::MAP_HUGETLB,
                        _ => MapFlags::empty(),
                    },
                -1,
                0,
            )
        }
        .map_err(|_| Error::from(ErrorKind::MmapFailed))?;

        region.mmap_start = PhysAddr::new(mmap_start as u64);

        if let VmMemBackingSrcType::VM_MEM_SRC_ANONYMOUS_THP = src_type {
            region.host_mem = region.mmap_start.align_up(huge_page_size as u64);
        } else {
            region.host_mem = region.mmap_start;
        }

        use nix::sys::mman::MmapAdvise;

        match src_type {
            VmMemBackingSrcType::VM_MEM_SRC_ANONYMOUS => unsafe {
                madvise(
                    region.host_mem.as_u64() as *mut _,
                    (npages * self.page_size as u64) as usize,
                    MmapAdvise::MADV_NOHUGEPAGE,
                )
                .map_err(|_| Error::from(ErrorKind::MadviseFailed))?;
            },
            VmMemBackingSrcType::VM_MEM_SRC_ANONYMOUS_THP => unsafe {
                nix::sys::mman::madvise(
                    region.host_mem.as_u64() as *mut _,
                    (npages * self.page_size as u64) as usize,
                    MmapAdvise::MADV_HUGEPAGE,
                )
                .map_err(|_| Error::from(ErrorKind::MadviseFailed))?;
            },
            VmMemBackingSrcType::VM_MEM_SRC_ANONYMOUS_HUGETLB => { /* FIXME: no madvise? */ }
        }

        region.region.slot = slot;
        region.region.flags = flags;
        region.region.guest_phys_addr = guest_paddr.as_u64();
        region.region.memory_size = npages * self.page_size;
        region.region.userspace_addr = region.host_mem.as_u64();

        unsafe {
            self.kvm_fd
                .set_user_memory_region(region.region)
                .map_err(map_context!())?
        };

        self.frame_allocator.memory_map.add_region(MemoryRegion {
            range: FrameRange::new(region.region.guest_phys_addr, region.region.memory_size),
            region_type: MemoryRegionType::Usable,
        });

        let zero_frame: PhysFrame = PhysFrame::from_start_address(PhysAddr::new(0)).unwrap();

        self.frame_allocator.mark_allocated_region(MemoryRegion {
            range: frame_range(PhysFrame::range(zero_frame, zero_frame + 1)),
            region_type: MemoryRegionType::FrameZero,
        });

        self.userspace_mem_regions.push(region);

        Ok(())
    }

    pub fn addr_gpa2hva(&self, gpa: PhysAddr) -> Result<HostVirtAddr, Error> {
        for region in &self.userspace_mem_regions {
            if (gpa.as_u64() >= region.region.guest_phys_addr)
                && (gpa.as_u64() <= (region.region.guest_phys_addr + region.region.memory_size - 1))
            {
                return Ok(HostVirtAddr::new(
                    region.host_mem.as_u64() + (gpa.as_u64() - region.region.guest_phys_addr),
                ));
            }
        }
        Err(ErrorKind::NoMappingForVirtualAddress.into())
    }

    pub fn addr_gva2gpa(&self, gva: VirtAddr) -> Result<PhysAddr, Error> {
        let pgd = self
            .pgd
            .ok_or_else(|| Error::from(ErrorKind::NoMappingForVirtualAddress))?;

        let index: [usize; 4] = [
            (gva.as_u64() >> 12) as usize & 0x1ffusize,
            (gva.as_u64() >> 21) as usize & 0x1ffusize,
            (gva.as_u64() >> 30) as usize & 0x1ffusize,
            (gva.as_u64() >> 39) as usize & 0x1ffusize,
        ];
        let pml4e = self.addr_gpa2hva(pgd)?;
        let pml4e = unsafe {
            core::slice::from_raw_parts(pml4e.as_u64() as *mut PageTableFlags, 512 as usize)
        };

        if !pml4e[index[3]].contains(PageTableFlags::PRESENT) {
            return Err(ErrorKind::NoMappingForVirtualAddress.into());
        }

        let pdpe = self.addr_gpa2hva(PhysAddr::new(pml4e[index[3]].addr() * self.page_size))?;
        let pdpe = unsafe {
            core::slice::from_raw_parts(pdpe.as_u64() as *mut PageTableFlags, 512 as usize)
        };

        if !pdpe[index[2]].contains(PageTableFlags::PRESENT) {
            return Err(ErrorKind::NoMappingForVirtualAddress.into());
        }

        let pde = self.addr_gpa2hva(PhysAddr::new(pdpe[index[2]].addr() * self.page_size))?;
        let pde = unsafe {
            core::slice::from_raw_parts(pde.as_u64() as *mut PageTableFlags, 512 as usize)
        };

        if !pde[index[1]].contains(PageTableFlags::PRESENT) {
            return Err(ErrorKind::NoMappingForVirtualAddress.into());
        }

        let pte = self.addr_gpa2hva(PhysAddr::new(pde[index[1]].addr() * self.page_size))?;
        let pte = unsafe {
            core::slice::from_raw_parts(pte.as_u64() as *mut PageTableFlags, 512 as usize)
        };

        if !pte[index[0]].contains(PageTableFlags::PRESENT) {
            return Err(ErrorKind::NoMappingForVirtualAddress.into());
        }

        Ok(PhysAddr::new(
            (pte[index[0]].addr() * self.page_size) + (gva.as_u64() & 0xfffu64),
        ))
    }

    pub fn virt_pg_map(
        &mut self,
        vaddr: VirtAddr,
        paddr: PhysAddr,
        pgd_memslot: u32,
    ) -> Result<(), Error> {
        let pgd = self
            .pgd
            .ok_or_else(|| Error::from(ErrorKind::NoMappingForVirtualAddress))?;

        /* FIXME:
            TEST_ASSERT((vaddr % vm->page_size) == 0,
                "Virtual address not on page boundary,\n"
                "  vaddr: 0x%lx vm->page_size: 0x%x",
                vaddr, vm->page_size);
            TEST_ASSERT(sparsebit_is_set(vm->vpages_valid,
                (vaddr >> vm->page_shift)),
                "Invalid virtual address, vaddr: 0x%lx",
                vaddr);
            TEST_ASSERT((paddr % vm->page_size) == 0,
                "Physical address not on page boundary,\n"
                "  paddr: 0x%lx vm->page_size: 0x%x",
                paddr, vm->page_size);
            TEST_ASSERT((paddr >> vm->page_shift) <= vm->max_gfn,
                "Physical address beyond beyond maximum supported,\n"
                "  paddr: 0x%lx vm->max_gfn: 0x%lx vm->page_size: 0x%x",
                paddr, vm->max_gfn, vm->page_size);
        */

        let index: [usize; 4] = [
            (vaddr.as_u64() >> 12) as usize & 0x1ffusize,
            (vaddr.as_u64() >> 21) as usize & 0x1ffusize,
            (vaddr.as_u64() >> 30) as usize & 0x1ffusize,
            (vaddr.as_u64() >> 39) as usize & 0x1ffusize,
        ];

        let pml4e = self.addr_gpa2hva(pgd)?;
        let pml4e = unsafe {
            core::slice::from_raw_parts_mut(pml4e.as_u64() as *mut PageTableFlags, 512 as usize)
        };

        if !pml4e[index[3]].contains(PageTableFlags::PRESENT) {
            pml4e[index[3]].insert(PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
            pml4e[index[3]].set_addr(
                self.vm_phy_page_alloc(
                    PhysAddr::new(KVM_GUEST_PAGE_TABLE_MIN_PADDR),
                    pgd_memslot,
                    MemoryRegionType::PageTable,
                )?
                .as_u64()
                    >> self.page_shift,
            );
        }

        let pdpe = self.addr_gpa2hva(PhysAddr::new(pml4e[index[3]].addr() * self.page_size))?;
        let pdpe = unsafe {
            core::slice::from_raw_parts_mut(pdpe.as_u64() as *mut PageTableFlags, 512 as usize)
        };

        if !pdpe[index[2]].contains(PageTableFlags::PRESENT) {
            pdpe[index[2]].insert(PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
            pdpe[index[2]].set_addr(
                self.vm_phy_page_alloc(
                    PhysAddr::new(KVM_GUEST_PAGE_TABLE_MIN_PADDR),
                    pgd_memslot,
                    MemoryRegionType::PageTable,
                )?
                .as_u64()
                    >> self.page_shift,
            );
        }

        let pde = self.addr_gpa2hva(PhysAddr::new(pdpe[index[2]].addr() * self.page_size))?;
        let pde = unsafe {
            core::slice::from_raw_parts_mut(pde.as_u64() as *mut PageTableFlags, 512 as usize)
        };

        if !pde[index[1]].contains(PageTableFlags::PRESENT) {
            pde[index[1]].insert(PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
            pde[index[1]].set_addr(
                self.vm_phy_page_alloc(
                    PhysAddr::new(KVM_GUEST_PAGE_TABLE_MIN_PADDR),
                    pgd_memslot,
                    MemoryRegionType::PageTable,
                )?
                .as_u64()
                    >> self.page_shift,
            );
        }

        let pte = self.addr_gpa2hva(PhysAddr::new(pde[index[1]].addr() * self.page_size))?;
        let pte = unsafe {
            core::slice::from_raw_parts_mut(pte.as_u64() as *mut PageTableFlags, 512 as usize)
        };
        pte[index[0]].set_addr(paddr.as_u64() >> self.page_shift);
        pte[index[0]].insert(PageTableFlags::PRESENT | PageTableFlags::WRITABLE);

        Ok(())
    }

    pub fn addr_gva2hva(&self, gva: VirtAddr) -> Result<HostVirtAddr, Error> {
        self.addr_gpa2hva(self.addr_gva2gpa(gva)?)
    }

    fn memslot2region(&self, memslot: u32) -> Result<usize, Error> {
        for (i, r) in self.userspace_mem_regions.iter().enumerate() {
            if r.region.slot == memslot {
                return Ok(i);
            }
        }
        Err(ErrorKind::NoMemRegionWithSlotFound.into())
    }

    pub fn vm_phy_pages_alloc(
        &mut self,
        num: usize,
        paddr_min: PhysAddr,
        memslot: u32,
        region_type: MemoryRegionType,
    ) -> Result<PhysAddr, Error> {
        assert!(num > 0);
        assert!((paddr_min.as_u64() % self.page_size) == 0);

        let region_index = self.memslot2region(memslot)?;

        let start = self.userspace_mem_regions[region_index]
            .used_phy_pages
            .find_unset_range((paddr_min.as_u64() >> self.page_shift as u64) as usize, num)
            .ok_or_else(|| Error::from(ErrorKind::NoMemFree))?;

        self.userspace_mem_regions[region_index]
            .used_phy_pages
            .set_range(start, num);

        let start = PhysAddr::new(start as u64 * self.page_size);
        let start_frame = PhysFrame::containing_address(start);
        let end_frame = PhysFrame::containing_address(start + num - 1u64);
        let memory_area = PhysFrame::range(start_frame, end_frame + 1);

        self.frame_allocator.mark_allocated_region(MemoryRegion {
            range: frame_range(memory_area),
            region_type,
        });

        Ok(start)
    }

    pub fn vm_phy_page_alloc(
        &mut self,
        paddr_min: PhysAddr,
        memslot: u32,
        region_type: MemoryRegionType,
    ) -> Result<PhysAddr, Error> {
        self.vm_phy_pages_alloc(1, paddr_min, memslot, region_type)
    }

    pub fn virt_pgd_alloc(&mut self, pgd_memslot: u32) -> Result<(), Error> {
        if self.pgd.is_none() {
            self.pgd = Some(self.vm_phy_page_alloc(
                PhysAddr::new(KVM_GUEST_PAGE_TABLE_MIN_PADDR),
                pgd_memslot,
                MemoryRegionType::PageTable,
            )?);
        }

        Ok(())
    }

    pub fn vm_vaddr_unused_gap(
        &mut self,
        sz: usize,
        vaddr_min: VirtAddr,
    ) -> Result<VirtAddr, Error> {
        let pages: usize = (sz + self.page_size as usize - 1) >> self.page_shift as usize;

        let mut pgidx_start: usize =
            ((vaddr_min.as_u64() + self.page_size - 1) >> self.page_shift as u64) as usize;

        if (pgidx_start * self.page_size as usize) < vaddr_min.as_u64() as usize {
            return Err(ErrorKind::NoVirtualAddressAvailable.into());
        }

        if !self.vpages_valid.is_set_num(pgidx_start, pages) {
            pgidx_start = match self.vpages_valid.next_set_num(pgidx_start, pages) {
                Some(start) => start,
                None => return Err(ErrorKind::NoVirtualAddressAvailable.into()),
            };
        }

        loop {
            if self.vpages_mapped.is_clear_num(pgidx_start, pages) {
                return Ok(VirtAddr::new(pgidx_start as u64 * self.page_size));
            }

            pgidx_start = self.vpages_mapped.next_clear_num(pgidx_start, pages);

            if !self.vpages_valid.is_set_num(pgidx_start, pages) {
                pgidx_start = match self.vpages_valid.next_set_num(pgidx_start, pages) {
                    Some(start) => start,
                    None => return Err(ErrorKind::NoVirtualAddressAvailable.into()),
                };
            }
        }
    }

    pub fn vm_vaddr_alloc(
        &mut self,
        sz: usize,
        vaddr_min: VirtAddr,
        data_memslot: u32,
        pgd_memslot: u32,
        region_type: MemoryRegionType,
    ) -> Result<VirtAddr, Error> {
        let mut pages: u64 = ((sz >> self.page_shift as usize) + {
            if (sz % self.page_size as usize) == 0 {
                0
            } else {
                1
            }
        }) as u64;

        self.virt_pgd_alloc(pgd_memslot)?;

        /*
         * Find an unused range of virtual page addresses of at least
         * pages in length.
         */
        let vaddr_start: VirtAddr = self.vm_vaddr_unused_gap(sz, vaddr_min)?;

        let mut vaddr = vaddr_start;

        /* Map the virtual pages. */
        loop {
            let paddr: PhysAddr = self.vm_phy_page_alloc(
                PhysAddr::new(KVM_UTIL_MIN_PFN * self.page_size),
                data_memslot,
                region_type,
            )?;
            self.virt_pg_map(vaddr, paddr, pgd_memslot)?;
            self.vpages_mapped
                .set((vaddr.as_u64() >> self.page_shift) as usize, true);
            pages -= 1;
            if pages == 0 {
                break;
            }
            vaddr += self.page_size;
        }

        Ok(vaddr_start)
    }

    pub fn elf_load(
        &mut self,
        program_invocation_name: &str,
        data_memslot: u32,
        pgd_memslot: u32,
        start_symbol: Option<&str>,
    ) -> Result<VirtAddr, Error> {
        use std::fs::File;
        use std::os::unix::io::AsRawFd;
        use xmas_elf::program::{self, ProgramHeader};
        use xmas_elf::sections;
        use xmas_elf::symbol_table::Entry;
        use xmas_elf::ElfFile;

        let file = File::open(program_invocation_name).map_err(map_context!())?;
        let mmap_size = file.metadata().map_err(map_context!())?.len() as usize;
        let mm = mmap::MemoryMap::new(
            mmap_size,
            &[
                mmap::MapOption::MapFd(file.as_raw_fd()),
                mmap::MapOption::MapReadable,
            ],
        )
        .map_err(|_| Error::from(ErrorKind::MmapFailed))?;

        let data = unsafe { core::slice::from_raw_parts(mm.data(), mmap_size) };

        let elf_file = ElfFile::new(data).map_err(map_context!())?;

        xmas_elf::header::sanity_check(&elf_file).map_err(map_context!())?;

        let mut guest_code: Option<VirtAddr> = None;

        match start_symbol {
            Some(start_symbol) => {
                let mut sect_iter = elf_file.section_iter();
                // Skip the first (dummy) section
                sect_iter.next();
                for sect in sect_iter {
                    sections::sanity_check(sect, &elf_file).unwrap();

                    if sect.get_type() == Ok(sections::ShType::SymTab) {
                        if let Ok(sections::SectionData::SymbolTable64(data)) =
                            sect.get_data(&elf_file)
                        {
                            for datum in data {
                                if datum.get_name(&elf_file).unwrap().eq(start_symbol) {
                                    guest_code = Some(VirtAddr::new(datum.value()));
                                }
                            }
                        } else {
                            unreachable!();
                        }
                    }
                }

                if guest_code.is_none() {
                    return Err(ErrorKind::GuestCodeNotFound.into());
                }
            }
            None => guest_code = Some(VirtAddr::new(elf_file.header.pt2.entry_point())),
        }

        for program_header in elf_file.program_iter() {
            match program_header {
                ProgramHeader::Ph64(header) => {
                    let segment = *header;
                    match segment.get_type().unwrap() {
                        program::Type::Load => {}
                        _ => continue,
                    }
                    let seg_vstart = segment.virtual_addr & (!(self.page_size - 1));
                    let seg_vend =
                        (segment.virtual_addr + segment.mem_size - 1) | (self.page_size - 1);
                    let seg_size = seg_vend - seg_vstart + 1;

                    let flags = segment.flags;
                    let mut page_table_flags = PageTableFlags::PRESENT;
                    if !flags.is_execute() {
                        page_table_flags |= PageTableFlags::NO_EXECUTE
                    };
                    if flags.is_write() {
                        page_table_flags |= PageTableFlags::WRITABLE
                    };

                    let vaddr = self.vm_vaddr_alloc(
                        seg_size as usize,
                        VirtAddr::new(seg_vstart),
                        data_memslot,
                        pgd_memslot,
                        MemoryRegionType::Kernel,
                    )?;

                    let seg = unsafe {
                        core::slice::from_raw_parts_mut(
                            self.addr_gva2hva(vaddr)?.as_u64() as *mut u8,
                            seg_size as usize,
                        )
                    };

                    seg[..segment.file_size as usize].copy_from_slice(
                        &data[segment.offset as usize
                            ..(segment.offset + segment.file_size) as usize],
                    );

                    for i in &mut seg[segment.file_size as _..] {
                        *i = 0;
                    }
                }
                ProgramHeader::Ph32(_) => panic!("does not support 32 bit elf files"),
            }
        }

        Ok(guest_code.unwrap())
    }

    fn kvm_setup_gdt(
        &mut self,
        dt: &mut kvm_dtable,
        gdt_memslot: u32,
        pgd_memslot: u32,
    ) -> Result<(), Error> {
        if self.gdt.is_none() {
            self.gdt = Some(self.vm_vaddr_alloc(
                self.page_size as usize,
                VirtAddr::new(KVM_UTIL_MIN_VADDR),
                gdt_memslot,
                pgd_memslot,
                MemoryRegionType::PageTable,
            )?);
        }

        dt.base = self.gdt.unwrap().as_u64();
        dt.limit = self.page_size as _;
        Ok(())
    }

    fn kvm_seg_fill_gdt_64bit(&self, segp: &mut kvm_segment) -> Result<(), Error> {
        let gdt = self.addr_gva2hva(self.gdt.unwrap())?;
        let desc: *mut gdt::desc64 =
            (gdt.as_u64() + (segp.selector as u64 >> 3u64) * 8u64) as *mut gdt::desc64;
        unsafe {
            (*desc).limit0 = (segp.limit & 0xFFFF) as _;
            (*desc).base0 = (segp.base & 0xFFFF) as _;
            (*desc).set_base1((segp.base >> 16) as u32);
            (*desc).set_s(segp.s.into());
            (*desc).set_type(segp.type_.into());
            (*desc).set_dpl(segp.dpl.into());
            (*desc).set_p(segp.present.into());
            (*desc).set_limit1(segp.limit >> 16);
            (*desc).set_l(segp.l.into());
            (*desc).set_db(segp.db.into());
            (*desc).set_g(segp.g.into());
            (*desc).set_base2((segp.base >> 24) as u32);
            if segp.s == 0 {
                (*desc).base3 = (segp.base >> 32) as u32;
            }
        }
        Ok(())
    }

    fn kvm_seg_set_kernel_code_64bit(
        &self,
        selector: u16,
        segp: &mut kvm_segment,
    ) -> Result<(), Error> {
        /* memset(segp, 0, sizeof(*segp)); */
        *segp = kvm_segment {
            base: 0,
            limit: 0xFFFF_FFFFu32,
            selector,
            type_: 0x08 | 0x01 | 0x02, // kFlagCode | kFlagCodeAccessed | kFlagCodeReadable
            present: 1,
            dpl: 0,
            db: 0,
            s: 0x1, // kTypeCodeData
            l: 1,
            g: 1,
            avl: 0,
            unusable: 0,
            padding: 0,
        };
        self.kvm_seg_fill_gdt_64bit(segp)?;
        Ok(())
    }

    fn kvm_seg_set_kernel_data_64bit(
        &self,
        selector: u16,
        segp: &mut kvm_segment,
    ) -> Result<(), Error> {
        /* memset(segp, 0, sizeof(*segp)); */
        *segp = kvm_segment {
            selector,
            limit: 0xFFFF_FFFFu32,
            s: 0x1,             // kTypeCodeData
            type_: 0x01 | 0x02, // kFlagData | kFlagDataAccessed | kFlagDataWritable
            g: 1,
            present: 1,
            base: 0,
            dpl: 0,
            db: 0,
            l: 0,
            avl: 0,
            unusable: 0,
            padding: 0,
        };
        self.kvm_seg_fill_gdt_64bit(segp)?;
        Ok(())
    }

    fn kvm_setup_tss_64bit(
        &mut self,
        segp: &mut kvm_segment,
        selector: u16,
        gdt_memslot: u32,
        pgd_memslot: u32,
    ) -> Result<(), Error> {
        if self.tss.is_none() {
            self.tss = Some(self.vm_vaddr_alloc(
                self.page_size as _,
                VirtAddr::new(KVM_UTIL_MIN_VADDR),
                gdt_memslot,
                pgd_memslot,
                MemoryRegionType::PageTable,
            )?);
        }

        *segp = kvm_segment {
            base: self.tss.unwrap().as_u64(),
            limit: 0x67,
            selector,
            type_: 0xb,
            present: 1,
            dpl: 0,
            db: 0,
            s: 0,
            l: 0,
            g: 0,
            avl: 0,
            unusable: 0,
            padding: 0,
        };

        self.kvm_seg_fill_gdt_64bit(segp)?;
        Ok(())
    }

    pub fn vcpu_setup(
        &mut self,
        vcpuid: u8,
        pgd_memslot: u32,
        gdt_memslot: u32,
    ) -> Result<(), Error> {
        let mut sregs = self.cpu_fd[vcpuid as usize]
            .get_sregs()
            .map_err(map_context!())?;

        sregs.idt.limit = 0;

        self.kvm_setup_gdt(&mut sregs.gdt, gdt_memslot, pgd_memslot)?;

        sregs.cr0 = (X86_CR0_PE | X86_CR0_NE | X86_CR0_PG) as u64;
        sregs.cr4 |= (X86_CR4_PAE | X86_CR4_OSFXSR) as u64;
        sregs.efer |= (EFER_LME | EFER_LMA | EFER_NX) as u64;

        // kvm_seg_set_unusable(&mut sregs.ldt);
        sregs.ldt = kvm_segment {
            base: 0,
            limit: 0,
            selector: 0,
            type_: 0,
            present: 0,
            dpl: 0,
            db: 0,
            s: 0,
            l: 0,
            g: 0,
            avl: 0,
            padding: 0,
            unusable: 1,
        };

        self.kvm_seg_set_kernel_code_64bit(0x8, &mut sregs.cs)?;
        self.kvm_seg_set_kernel_data_64bit(0x10, &mut sregs.ds)?;
        self.kvm_seg_set_kernel_data_64bit(0x10, &mut sregs.es)?;
        self.kvm_setup_tss_64bit(&mut sregs.tr, 0x18, gdt_memslot, pgd_memslot)?;

        sregs.cr3 = self.pgd.unwrap().as_u64();

        self.cpu_fd[vcpuid as usize]
            .set_sregs(&sregs)
            .map_err(map_context!())?;

        Ok(())
    }

    fn vcpu_add(&mut self, vcpuid: u8, pgd_memslot: u32, gdt_memslot: u32) -> Result<(), Error> {
        let vcpu_fd = self.kvm_fd.create_vcpu(vcpuid).map_err(map_context!())?;
        self.cpu_fd.insert(vcpuid as usize, vcpu_fd);
        self.vcpu_setup(vcpuid, pgd_memslot, gdt_memslot)?;

        Ok(())
    }

    fn vcpu_add_default(&mut self, vcpuid: u8, guest_code: VirtAddr) -> Result<(), Error> {
        let stack_vaddr: VirtAddr = self.vm_vaddr_alloc(
            (DEFAULT_STACK_PGS * self.page_size) as usize,
            VirtAddr::new(DEFAULT_GUEST_STACK_VADDR_MIN),
            0,
            0,
            MemoryRegionType::KernelStack,
        )?;

        let stack_vaddr_end = stack_vaddr.as_u64() + (DEFAULT_STACK_PGS * self.page_size);

        let physical_memory_offset = {
            // Map complete guest physical memory to PHYSICAL_MEMORY_OFFSET
            use x86_64::structures::paging::Page;

            let physical_memory_offset = PHYSICAL_MEMORY_OFFSET;

            let virt_for_phys = |phys: PhysAddr| -> VirtAddr {
                VirtAddr::new(phys.as_u64() + physical_memory_offset)
            };

            // FIXME: change to Size2MiB
            let start_frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(0));
            let end_frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(
                self.userspace_mem_regions[0].mmap_size as _,
            ));

            for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
                let page =
                    Page::<Size4KiB>::containing_address(virt_for_phys(frame.start_address()));
                let _flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
                self.virt_pg_map(page.start_address(), frame.start_address(), 0)?;
                /*
                unsafe {
                    page_table::map_page(
                        page,
                        frame,
                        flags,
                        &mut rec_page_table,
                        &mut frame_allocator,
                    )
                }
                    .expect("Mapping of bootinfo page failed")
                    .flush();
                    */
            }

            physical_memory_offset
        };

        let boot_info_vaddr: VirtAddr = self.vm_vaddr_alloc(
            self.page_size as _,
            VirtAddr::new(KVM_UTIL_MIN_VADDR),
            0,
            0,
            MemoryRegionType::BootInfo,
        )?;

        let mut boot_info = BootInfo::new(
            self.frame_allocator.memory_map.clone(), // FIXME: merge continuous regions
            self.pgd.unwrap().as_u64(),
            physical_memory_offset,
        );

        boot_info.memory_map.sort();
        // Write boot info to boot info page.
        let boot_info_addr = self.addr_gva2hva(boot_info_vaddr)?;
        //serial_println!("stage4: boot_info_addr={:#?}", boot_info);
        unsafe { boot_info_addr.as_mut_ptr::<BootInfo>().write(boot_info) };

        /* Create VCPU */
        self.vcpu_add(vcpuid, 0, 0)?;

        /* Setup guest general purpose registers */
        let mut regs = self.cpu_fd[vcpuid as usize]
            .get_regs()
            .map_err(map_context!())?;
        regs.rflags |= 0x2;
        regs.rsp = stack_vaddr_end;
        regs.rip = guest_code.as_u64();
        regs.rdi = boot_info_vaddr.as_u64();

        self.cpu_fd[vcpuid as usize]
            .set_regs(&regs)
            .map_err(map_context!())?;

        /* Setup the MP state */
        let mp_state: kvm_mp_state = kvm_mp_state { mp_state: 0 };
        self.cpu_fd[vcpuid as usize]
            .set_mp_state(mp_state)
            .map_err(map_context!())?;

        Ok(())
    }

    fn create_irqchip(&mut self) -> Result<(), Error> {
        self.kvm_fd.create_irq_chip().map_err(map_context!())?;
        self.has_irqchip = true;
        Ok(())
    }

    pub fn vm_create_default(
        program_invocation_name: &str,
        vcpuid: u8,
        extra_mem_pages: u64,
        entry_symbol: Option<&str>,
    ) -> Result<Self, Error> {
        let extra_pg_pages: u64 = extra_mem_pages / 512 * 2;

        /* Create VM */
        let mut vm = KvmVm::vm_create(DEFAULT_GUEST_PHY_PAGES + extra_pg_pages)?;

        /* Setup guest code */
        let guest_code = vm.elf_load(program_invocation_name, 0, 0, entry_symbol)?;

        /* Setup IRQ Chip */
        vm.create_irqchip()?;

        /* Add the first vCPU. */
        vm.vcpu_add_default(vcpuid, guest_code)?;

        /* Set CPUID */
        let cpuid = vm
            .kvm
            .get_supported_cpuid(MAX_KVM_CPUID_ENTRIES)
            .map_err(map_context!())?;

        vm.cpu_fd[vcpuid as usize]
            .set_cpuid2(&cpuid)
            .map_err(map_context!())?;

        Ok(vm)
    }
}
