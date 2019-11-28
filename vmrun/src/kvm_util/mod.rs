use kvm_bindings::{kvm_mp_state, kvm_segment, kvm_userspace_memory_region};
use kvm_ioctls::{Kvm, VcpuFd, VmFd, MAX_KVM_CPUID_ENTRIES};

pub use x86_64::{gdt, HostVirtAddr, PhysAddr, VirtAddr};
mod frame_allocator;
pub mod x86_64;

use crate::error::*;
use crate::{context, map_context};
use bootinfo::{
    BootInfo, FrameRange, MemoryMap, MemoryRegion, MemoryRegionType, PageTables,
    BOOTINFO_PHYS_ADDR, BOOT_GDT_OFFSET, BOOT_IDT_OFFSET, BOOT_STACK_POINTER,
    BOOT_STACK_POINTER_SIZE, HIMEM_START, PAGETABLE_LEN, PDE_START, PDPTE_OFFSET_START,
    PDPTE_START, PML4_START, SYSCALL_PHYS_ADDR,
};

use crate::kvm_util::gdt::{gdt_entry, kvm_segment_from_gdt};
use vmsyscall::{KvmSyscall, KvmSyscallRet};
use x86_64::consts::*;
use x86_64::structures::paging::{frame::PhysFrameRange, PhysFrame};

const DEFAULT_GUEST_MEM: u64 = 100 * 1024 * 1024;
const DEFAULT_GUEST_PAGE_SIZE: usize = 4096;
const PHYSICAL_MEMORY_OFFSET: u64 = 0xFFFF_8000_0000_0000;

struct UserspaceMemRegion {
    region: kvm_userspace_memory_region,
    host_mem: HostVirtAddr,
    mmap_start: HostVirtAddr,
    mmap_size: usize,
}

pub struct KvmVm {
    kvm: Kvm,
    pub cpu_fd: Vec<VcpuFd>,
    kvm_fd: VmFd,
    page_size: usize,
    frame_allocator: frame_allocator::FrameAllocator,
    userspace_mem_regions: Vec<UserspaceMemRegion>,
    has_irqchip: bool,
    pub syscall_hostvaddr: Option<HostVirtAddr>,
}

fn frame_range(range: PhysFrameRange) -> FrameRange {
    FrameRange::new(
        range.start.start_address().as_u64(),
        range.end.start_address().as_u64(),
    )
}

impl KvmVm {
    pub fn vm_create(phy_pages: u64) -> Result<Self, Error> {
        let kvm = Kvm::new().unwrap();

        let kvm_fd: VmFd = kvm.create_vm().map_err(map_context!())?;

        let mut vm = KvmVm {
            kvm,
            cpu_fd: vec![],
            kvm_fd,
            page_size: DEFAULT_GUEST_PAGE_SIZE,
            frame_allocator: frame_allocator::FrameAllocator {
                memory_map: MemoryMap::new(),
            },
            userspace_mem_regions: vec![],
            has_irqchip: false,
            syscall_hostvaddr: None,
        };

        //FIXME: remove phy_pages
        if phy_pages != 0 {
            vm.vm_userspace_mem_region_add(PhysAddr::new(0), 0, phy_pages, 0)?;

            let zero_frame: PhysFrame = PhysFrame::from_start_address(PhysAddr::new(0)).unwrap();
            let page_table_frame: PhysFrame =
                PhysFrame::from_start_address(PhysAddr::new(PML4_START as _)).unwrap();

            vm.frame_allocator.mark_allocated_region(MemoryRegion {
                range: frame_range(PhysFrame::range(zero_frame, zero_frame + 1)),
                region_type: MemoryRegionType::FrameZero,
            });

            vm.frame_allocator.mark_allocated_region(MemoryRegion {
                range: frame_range(PhysFrame::range(zero_frame + 1, page_table_frame)),
                region_type: MemoryRegionType::Reserved,
            });

            vm.frame_allocator.mark_allocated_region(MemoryRegion {
                range: frame_range(PhysFrame::range(
                    page_table_frame,
                    page_table_frame + PAGETABLE_LEN / vm.page_size as u64,
                )),
                region_type: MemoryRegionType::PageTable,
            });

            let bootinfo_frame: PhysFrame =
                PhysFrame::from_start_address(PhysAddr::new(BOOTINFO_PHYS_ADDR)).unwrap();
            vm.frame_allocator.mark_allocated_region(MemoryRegion {
                range: frame_range(PhysFrame::range(bootinfo_frame, bootinfo_frame + 1)),
                region_type: MemoryRegionType::BootInfo,
            });

            let syscall_frame: PhysFrame =
                PhysFrame::from_start_address(PhysAddr::new(SYSCALL_PHYS_ADDR)).unwrap();
            vm.frame_allocator.mark_allocated_region(MemoryRegion {
                range: frame_range(PhysFrame::range(syscall_frame, syscall_frame + 1)),
                region_type: MemoryRegionType::SysCall,
            });

            // FIXME: add stack guard page
            let stack_frame: PhysFrame = PhysFrame::from_start_address(PhysAddr::new(
                BOOT_STACK_POINTER - BOOT_STACK_POINTER_SIZE,
            ))
            .unwrap();

            if syscall_frame + 1 < stack_frame {
                vm.frame_allocator.mark_allocated_region(MemoryRegion {
                    range: frame_range(PhysFrame::range(syscall_frame + 1, stack_frame)),
                    region_type: MemoryRegionType::Reserved,
                });
            }

            let stack_frame_end: PhysFrame =
                PhysFrame::from_start_address(PhysAddr::new(HIMEM_START as u64)).unwrap();

            vm.frame_allocator.mark_allocated_region(MemoryRegion {
                range: frame_range(PhysFrame::range(stack_frame, stack_frame_end)),
                region_type: MemoryRegionType::KernelStack,
            });

            vm.setup_page_tables()?;
        }

        Ok(vm)
    }

    pub fn vm_userspace_mem_region_add(
        &mut self,
        guest_paddr: PhysAddr,
        slot: u32,
        npages: u64,
        flags: u32,
    ) -> Result<(), Error> {
        for r in self.userspace_mem_regions.iter() {
            if r.region.slot == slot {
                return Err(context!(ErrorKind::MemRegionWithSlotAlreadyExists));
            }

            if guest_paddr.as_u64() <= (r.region.guest_phys_addr + r.region.memory_size)
                && (guest_paddr.as_u64() + npages * self.page_size as u64)
                    >= r.region.guest_phys_addr
            {
                return Err(context!(ErrorKind::OverlappingUserspaceMemRegionExists));
            }
        }

        let mut region = UserspaceMemRegion {
            region: Default::default(),
            host_mem: HostVirtAddr::new(0),
            mmap_start: HostVirtAddr::new(0),
            mmap_size: (npages * self.page_size as u64) as _,
        };
        let mm = mmap::MemoryMap::new(
            region.mmap_size,
            &[mmap::MapOption::MapReadable, mmap::MapOption::MapWritable],
        )
        .map_err(|_| context!(ErrorKind::MmapFailed))?;
        let mmap_start = mm.data();
        // FIXME: No drop for mm
        std::mem::forget(mm);

        region.mmap_start = HostVirtAddr::new(mmap_start as u64);

        region.host_mem = region.mmap_start;

        region.region.slot = slot;
        region.region.flags = flags;
        region.region.guest_phys_addr = guest_paddr.as_u64();
        region.region.memory_size = npages * self.page_size as u64;
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

        self.userspace_mem_regions.push(region);

        Ok(())
    }

    pub fn addr_gpa2hva(&self, guest_phys_addr: PhysAddr) -> Result<HostVirtAddr, Error> {
        for region in &self.userspace_mem_regions {
            if (guest_phys_addr.as_u64() >= region.region.guest_phys_addr)
                && (guest_phys_addr.as_u64()
                    <= (region.region.guest_phys_addr + region.region.memory_size - 1))
            {
                return Ok(HostVirtAddr::new(
                    region.host_mem.as_u64()
                        + (guest_phys_addr.as_u64() - region.region.guest_phys_addr),
                ));
            }
        }
        Err(context!(ErrorKind::NoMappingForVirtualAddress))
    }

    fn setup_page_tables(&mut self) -> Result<(), Error> {
        let mut page_tables = PageTables::default();

        // Puts PML4 right after zero page but aligned to 4k.
        let boot_pdpte_addr = PDPTE_START;
        let mut boot_pde_addr = PDE_START;
        let boot_pdpte_offset_addr = PDPTE_OFFSET_START;

        // Entry covering VA [0..512GB)
        page_tables.pml4t[0] = boot_pdpte_addr as u64 | 0x3;

        // Entry covering VA [0..512GB) with physical offset PHYSICAL_MEMORY_OFFSET
        page_tables.pml4t[(PHYSICAL_MEMORY_OFFSET >> 39) as usize & 0x1FFusize] =
            boot_pdpte_offset_addr as u64 | 0x3;

        // Entries covering VA [0..4GB)
        for i_g in 0..4 {
            // Entry covering VA [i..i+1GB)
            page_tables.pml3t_ident[i_g] = boot_pde_addr as u64 | 0x3;
            // 512 2MB entries together covering VA [i*1GB..(i+1)*1GB). Note we are assuming
            // CPU supports 2MB pages (/proc/cpuinfo has 'pse'). All modern CPUs do.
            for i in i_g * 512..(i_g + 1) * 512 {
                page_tables.pml2t_ident[i] = ((i as u64) << 21) | 0x83u64;
            }
            boot_pde_addr += 0x1000;
        }

        // Entry covering VA [0..512GB) with physical offset PHYSICAL_MEMORY_OFFSET
        for i in 0..512 {
            page_tables.pml3t_offset[i] = ((i as u64) << 30) | 0x83u64;
        }

        let guest_pg_addr: *mut PageTables = self
            .addr_gpa2hva(PhysAddr::new(PML4_START as _))?
            .as_mut_ptr();

        unsafe {
            // FIXME: SEV LOAD
            guest_pg_addr.write(page_tables);
        }

        Ok(())
    }

    pub fn elf_load(
        &mut self,
        program_invocation_name: &str,
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
        .map_err(|_| context!(ErrorKind::MmapFailed))?;

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
                    return Err(context!(ErrorKind::GuestCodeNotFound));
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

                    let start_phys = PhysAddr::new(segment.virtual_addr);
                    let start_frame: PhysFrame =
                        PhysFrame::from_start_address(start_phys.align_down(self.page_size as u64))
                            .unwrap();

                    let end_frame: PhysFrame = PhysFrame::from_start_address(
                        PhysAddr::new(segment.virtual_addr + segment.mem_size - 1)
                            .align_up(self.page_size as u64),
                    )
                    .unwrap();

                    let region = MemoryRegion {
                        range: frame_range(PhysFrame::range(start_frame, end_frame)),
                        region_type: MemoryRegionType::Kernel,
                    };

                    self.frame_allocator.mark_allocated_region(region);

                    // FIXME: SEV LOAD
                    let host_slice = unsafe {
                        core::slice::from_raw_parts_mut(
                            self.addr_gpa2hva(start_phys)?.as_u64() as *mut u8,
                            segment.mem_size as usize,
                        )
                    };

                    host_slice[..segment.file_size as usize].copy_from_slice(
                        &data[segment.offset as usize
                            ..(segment.offset + segment.file_size) as usize],
                    );

                    host_slice[segment.file_size as _..]
                        .iter_mut()
                        .for_each(|i| *i = 0);
                }
                ProgramHeader::Ph32(_) => panic!("does not support 32 bit elf files"),
            }
        }

        Ok(guest_code.unwrap())
    }

    fn write_gdt_table(&self, table: &[u64]) -> Result<(), Error> {
        let gdt_addr: *mut u64 = self
            .addr_gpa2hva(PhysAddr::new(BOOT_GDT_OFFSET as _))?
            .as_mut_ptr();
        for (index, entry) in table.iter().enumerate() {
            let addr = unsafe { gdt_addr.offset(index as _) };
            unsafe { addr.write(*entry) };
        }
        Ok(())
    }

    fn write_idt_value(&self, val: u64) -> Result<(), Error> {
        let boot_idt_addr: *mut u64 = self
            .addr_gpa2hva(PhysAddr::new(BOOT_IDT_OFFSET as _))?
            .as_mut_ptr();
        unsafe { boot_idt_addr.write(val) }
        Ok(())
    }

    pub fn vcpu_setup(&mut self, vcpuid: u8) -> Result<(), Error> {
        let mut sregs = self.cpu_fd[vcpuid as usize]
            .get_sregs()
            .map_err(map_context!())?;

        let gdt_table: [u64; 4] = [
            gdt_entry(0, 0, 0),            // NULL
            gdt_entry(0xa09b, 0, 0xfffff), // CODE
            gdt_entry(0xc093, 0, 0xfffff), // DATA
            gdt_entry(0x808b, 0, 0xfffff), // TSS
        ];

        let code_seg = kvm_segment_from_gdt(gdt_table[1], 1);
        let data_seg = kvm_segment_from_gdt(gdt_table[2], 2);
        let tss_seg = kvm_segment_from_gdt(gdt_table[3], 3);

        // Write segments
        self.write_gdt_table(&gdt_table[..])?;
        sregs.gdt.base = BOOT_GDT_OFFSET as u64;
        sregs.gdt.limit = core::mem::size_of_val(&gdt_table) as u16 - 1;

        self.write_idt_value(0)?;
        sregs.idt.base = BOOT_IDT_OFFSET as u64;
        sregs.idt.limit = core::mem::size_of::<u64>() as u16 - 1;

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

        sregs.cs = code_seg;
        sregs.ds = data_seg;
        sregs.es = data_seg;
        //sregs.fs = data_seg;
        //sregs.gs = data_seg;
        //sregs.ss = data_seg; // FIXME: double fault in exception handler
        sregs.tr = tss_seg;

        sregs.cr0 = (X86_CR0_PE | X86_CR0_NE | X86_CR0_PG) as u64;
        sregs.cr4 |= (X86_CR4_PAE | X86_CR4_OSFXSR) as u64;
        sregs.efer |= (EFER_LME | EFER_LMA | EFER_NX) as u64;

        sregs.cr3 = PML4_START as _;

        self.cpu_fd[vcpuid as usize]
            .set_sregs(&sregs)
            .map_err(map_context!())?;

        Ok(())
    }

    fn vcpu_add(&mut self, vcpuid: u8) -> Result<(), Error> {
        let vcpu_fd = self.kvm_fd.create_vcpu(vcpuid).map_err(map_context!())?;
        self.cpu_fd.insert(vcpuid as usize, vcpu_fd);
        self.vcpu_setup(vcpuid)?;

        Ok(())
    }

    fn vcpu_add_default(&mut self, vcpuid: u8, guest_code: VirtAddr) -> Result<(), Error> {
        let boot_info_vaddr = PhysAddr::new(BOOTINFO_PHYS_ADDR);
        let syscall_vaddr = PhysAddr::new(SYSCALL_PHYS_ADDR);

        self.syscall_hostvaddr = Some(self.addr_gpa2hva(syscall_vaddr)?);

        let mut boot_info = BootInfo::new(
            self.frame_allocator.memory_map.clone(),
            PML4_START as _,
            PHYSICAL_MEMORY_OFFSET,
        );

        boot_info.memory_map.sort();
        // Write boot info to boot info page.
        let boot_info_addr = self.addr_gpa2hva(boot_info_vaddr)?;
        //serial_println!("stage4: boot_info_addr={:#?}", boot_info);
        unsafe { boot_info_addr.as_mut_ptr::<BootInfo>().write(boot_info) };

        /* Create VCPU */
        self.vcpu_add(vcpuid)?;

        /* Setup guest general purpose registers */
        let mut regs = self.cpu_fd[vcpuid as usize]
            .get_regs()
            .map_err(map_context!())?;
        regs.rflags |= 0x2;
        regs.rsp = BOOT_STACK_POINTER;
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

    pub fn handle_syscall(&self, syscall: KvmSyscall) -> KvmSyscallRet {
        match syscall {
            KvmSyscall::Mmap {
                addr: _,
                len: _,
                prot: _,
                flags: _,
            } => {
                /*
                                let ret = unsafe {
                                    mmap(
                                        null_mut(),
                                        len,
                                        ProtFlags::from_bits_truncate(prot),
                                        MapFlags::from_bits_truncate(flags),
                                        -1,
                                        0,
                                    )
                                };
                                let mmap_start = match ret {
                                    Err(nix::Error::Sys(e)) if e == nix::errno::Errno::ENOMEM => {
                                        return KvmSyscallRet::Mmap(Err(vmsyscall::Error::ENOMEM))
                                    }
                                    Err(_) => return KvmSyscallRet::Mmap(Err(vmsyscall::Error::OTHERERROR)),
                                    Ok(v) => v,
                                };
                */
                return KvmSyscallRet::Mmap(Err(vmsyscall::Error::OTHERERROR));
                /*
                let mut region = UserspaceMemRegion {
                    region: Default::default(),
                    used_phy_pages: Default::default(),
                    host_mem: PhysAddr::new(mmap_start as u64),
                    mmap_start: PhysAddr::new(mmap_start as u64),
                    mmap_size: len as _,
                };

                region.region.slot = 0;
                region.region.flags = flags as _;
                region.region.guest_phys_addr = addr as _;
                region.region.memory_size = len as _;
                region.region.userspace_addr = region.host_mem.as_u64();

                unsafe {
                    self.kvm_fd
                        .set_user_memory_region(region.region)
                        .map_err(map_context!())?
                };

                //self.userspace_mem_regions.push(region);

                KvmSyscallRet::Mmap(Ok(region.mmap_start.as_u64() as _))
                */
            }
            KvmSyscall::Madvise {
                addr: _,
                len: _,
                advice: _,
            } => KvmSyscallRet::Madvise(Err(vmsyscall::Error::OTHERERROR)),
            KvmSyscall::Mremap {
                addr: _,
                len: _,
                new_len: _,
                flags: _,
            } => KvmSyscallRet::Mremap(Err(vmsyscall::Error::OTHERERROR)),
            KvmSyscall::Munmap { addr: _, len: _ } => {
                KvmSyscallRet::Munmap(Err(vmsyscall::Error::OTHERERROR))
            }
            KvmSyscall::Mprotect {
                addr: _,
                len: _,
                prot: _,
            } => KvmSyscallRet::Mprotect(Err(vmsyscall::Error::OTHERERROR)),
        }
    }

    fn create_irqchip(&mut self) -> Result<(), Error> {
        self.kvm_fd.create_irq_chip().map_err(map_context!())?;
        self.has_irqchip = true;
        Ok(())
    }

    pub fn vm_create_default(
        program_invocation_name: &str,
        vcpuid: u8,
        entry_symbol: Option<&str>,
    ) -> Result<Self, Error> {
        /* Create VM */
        let mut vm = KvmVm::vm_create((DEFAULT_GUEST_MEM / DEFAULT_GUEST_PAGE_SIZE as u64) as _)?;

        /* Setup guest code */
        let guest_code = vm.elf_load(program_invocation_name, entry_symbol)?;

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
