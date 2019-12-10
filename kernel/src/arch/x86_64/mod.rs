#[macro_use]
pub mod serial;
pub mod gdt;
pub mod interrupts;
pub mod pti;
pub mod syscall;

use crate::memory::BootInfoFrameAllocator;
use alloc::alloc::{GlobalAlloc, Layout};
use boot::layout::{USER_STACK_OFFSET, USER_STACK_SIZE};
use boot::{BootInfo, MemoryRegionType};
use core::ptr::null_mut;
use x86_64::structures::paging::OffsetPageTable;
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    PhysAddr, VirtAddr,
};
use xmas_elf::program::{self, ProgramHeader64};

const PAGESIZE: usize = 4096;
pub fn pagesize() -> usize {
    PAGESIZE
}

pub const HEAP_START: usize = 0x4E43_0000_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB
pub const STACK_START: usize = 0x4848_0000_0000;
pub const STACK_SIZE: usize = 1024 * 1024; // 1MiB

extern "C" {
    static _app_start_addr: usize;
    static _app_size: usize;
}

pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    }

    unsafe {
        crate::ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}

pub fn init_stack(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError> {
    let stack_start = VirtAddr::new(STACK_START as u64);
    let stack_end = stack_start + STACK_SIZE - 1u64;
    let stack_start_page = Page::containing_address(stack_start);
    let stack_end_page = Page::containing_address(stack_end);

    let page_range = { Page::range_inclusive(stack_start_page + 1, stack_end_page) };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    }

    // Guard Page
    let frame = frame_allocator
        .allocate_frame()
        .ok_or(MapToError::FrameAllocationFailed)?;
    let flags = PageTableFlags::PRESENT;
    unsafe {
        mapper
            .map_to(stack_start_page, frame, flags, frame_allocator)?
            .flush()
    };

    unsafe {
        use x86_64::instructions::tables::load_tss;
        gdt::GDT.as_ref().unwrap().0.load();
        gdt::TSS.as_mut().unwrap().privilege_stack_table[0] = stack_end;
        load_tss(gdt::GDT.as_ref().unwrap().1.tss_selector);
        gdt::GDT.as_ref().unwrap().0.load();
    }

    Ok(())
}

pub struct Dummy;

unsafe impl GlobalAlloc for Dummy {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        panic!("dealloc should be never called")
    }
}

static mut ENTRY_POINT: Option<
    fn(mapper: &mut OffsetPageTable, frame_allocator: &mut BootInfoFrameAllocator) -> !,
> = None;
static mut FRAME_ALLOCATOR: Option<BootInfoFrameAllocator> = None;
static mut MAPPER: Option<OffsetPageTable> = None;

pub fn init(
    boot_info: &'static mut BootInfo,
    entry_point: fn(
        mapper: &mut OffsetPageTable,
        frame_allocator: &mut BootInfoFrameAllocator,
    ) -> !,
) -> ! {
    gdt::init();
    unsafe { syscall::init() };
    interrupts::init_idt();
    x86_64::instructions::interrupts::enable();
    //println!("{:#?}", boot_info);

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);

    unsafe { MAPPER.replace(crate::memory::init(phys_mem_offset)) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&mut boot_info.memory_map) };

    init_heap(unsafe { MAPPER.as_mut().unwrap() }, &mut frame_allocator)
        .expect("heap initialization failed");

    init_stack(unsafe { MAPPER.as_mut().unwrap() }, &mut frame_allocator)
        .expect("heap initialization failed");

    unsafe {
        FRAME_ALLOCATOR.replace(frame_allocator);
        ENTRY_POINT.replace(entry_point);
    }

    unsafe { crate::context_switch(init_after_stack_swap, STACK_START + STACK_SIZE) }
}

fn init_after_stack_swap() -> ! {
    let mut frame_allocator = unsafe { FRAME_ALLOCATOR.take().unwrap() };
    let mapper = unsafe { MAPPER.as_mut().unwrap() };
    let entry_point = unsafe { ENTRY_POINT.take().unwrap() };

    frame_allocator.set_region_type_usable(MemoryRegionType::KernelStack);

    entry_point(mapper, &mut frame_allocator)
}

pub fn exec_app(mapper: &mut OffsetPageTable, frame_allocator: &mut BootInfoFrameAllocator) -> ! {
    use xmas_elf::program::ProgramHeader;

    let sp = USER_STACK_OFFSET + USER_STACK_SIZE - 256;
    println!("USER_STACK_OFFSET={:#X}", USER_STACK_OFFSET);
    let sp_page = Page::containing_address(VirtAddr::new(USER_STACK_OFFSET as u64));
    let frame = frame_allocator.allocate_frame().unwrap();

    unsafe {
        mapper
            .map_to(
                sp_page,
                frame,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE,
                frame_allocator,
            )
            .unwrap()
            .flush();
    }

    // Extract required information from the ELF file.
    let entry_point;
    let app_start_ptr = unsafe { &_app_start_addr as *const _ as u64 };
    unsafe {
        println!("app start {:#X}", app_start_ptr);
        println!("app size {:#X}", &_app_size as *const _ as u64);
    }
    let kernel = unsafe {
        core::slice::from_raw_parts(
            &_app_start_addr as *const _ as *const u8,
            &_app_size as *const _ as usize,
        )
    };
    let elf_file = xmas_elf::ElfFile::new(kernel).unwrap();
    xmas_elf::header::sanity_check(&elf_file).unwrap();

    entry_point = elf_file.header.pt2.entry_point();

    for program_header in elf_file.program_iter() {
        match program_header {
            ProgramHeader::Ph64(header) => {
                let segment = *header;
                map_user_segment(
                    &segment,
                    PhysAddr::new(app_start_ptr),
                    mapper,
                    frame_allocator,
                )
                .unwrap();
                //println!("{:#?}", segment);
            }
            ProgramHeader::Ph32(_) => panic!("does not support 32 bit elf files"),
        }
    }
    println!("app_entry_point={:#X}", entry_point);
    println!("stackpointer={:#X}", sp);

    unsafe {
        syscall::usermode(entry_point as usize, sp, 0);
    }
}

pub(crate) fn map_user_segment(
    segment: &ProgramHeader64,
    file_start: PhysAddr,
    page_table: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError> {
    let typ = segment.get_type().unwrap();
    match typ {
        program::Type::Load => {
            let mem_size = segment.mem_size;
            let file_size = segment.file_size;
            let file_offset = segment.offset;
            let phys_start_addr = file_start + file_offset;
            let virt_start_addr = VirtAddr::new(segment.virtual_addr);

            let start_page: Page = Page::containing_address(virt_start_addr);
            let end_page: Page = Page::containing_address(virt_start_addr + file_size - 1u64);
            let page_range = Page::range_inclusive(start_page, end_page);
            //println!("{:#?}", page_range);

            let flags = segment.flags;
            let mut page_table_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
            if !flags.is_execute() {
                page_table_flags |= PageTableFlags::NO_EXECUTE
            };
            if flags.is_write() {
                page_table_flags |= PageTableFlags::WRITABLE
            };

            for page in page_range {
                let frame = frame_allocator
                    .allocate_frame()
                    .ok_or(MapToError::FrameAllocationFailed)?;
                println!(
                    "map {:#X} to {:#X}",
                    frame.start_address().as_u64(),
                    page.start_address().as_u64(),
                );
                unsafe {
                    page_table
                        .map_to(
                            page,
                            frame,
                            page_table_flags | PageTableFlags::WRITABLE,
                            frame_allocator,
                        )?
                        .flush()
                };
            }
            unsafe {
                let src = core::slice::from_raw_parts(
                    phys_start_addr.as_u64() as *const u8,
                    file_size as _,
                );
                let dst = core::slice::from_raw_parts_mut(
                    virt_start_addr.as_mut_ptr::<u8>(),
                    file_size as _,
                );
                dst.copy_from_slice(src);
            }
            for page in page_range {
                page_table
                    .update_flags(page, page_table_flags)
                    .unwrap()
                    .flush();
                println!(
                    "updating flags for {:#X}  {:#?}",
                    page.start_address().as_u64(),
                    page_table_flags
                );
            }

            if mem_size > file_size {
                panic!("mem_size > file_size");
                #[cfg(CLEAR_BSS)]
                {
                    // FIXME: allocate
                    // .bss section (or similar), which needs to be zeroed
                    let zero_start = virt_start_addr + file_size;
                    let zero_end = virt_start_addr + mem_size;
                    if zero_start.as_u64() & 0xfff != 0 {
                        // A part of the last mapped frame needs to be zeroed. This is
                        // not possible since it could already contains parts of the next
                        // segment. Thus, we need to copy it before zeroing.

                        // TODO: search for a free page dynamically
                        let temp_page: Page =
                            Page::containing_address(VirtAddr::new(0xfeeefeee000));
                        let new_frame = frame_allocator
                            .allocate_frame()
                            .ok_or(MapToError::FrameAllocationFailed)?;

                        unsafe {
                            page_table.map_to(
                                temp_page.clone(),
                                new_frame.clone(),
                                page_table_flags,
                                frame_allocator,
                            )?
                        }
                        .flush();

                        type PageArray = [u64; Size4KiB::SIZE as usize / 8];

                        let last_page =
                            Page::containing_address(virt_start_addr + file_size - 1u64);
                        let last_page_ptr = last_page.start_address().as_ptr::<PageArray>();
                        let temp_page_ptr = temp_page.start_address().as_mut_ptr::<PageArray>();

                        unsafe {
                            // copy contents
                            temp_page_ptr.write(last_page_ptr.read());
                        }

                        // remap last page
                        if let Err(e) = page_table.unmap(last_page.clone()) {
                            return Err(match e {
                                UnmapError::ParentEntryHugePage => MapToError::ParentEntryHugePage,
                                UnmapError::PageNotMapped => unreachable!(),
                                UnmapError::InvalidFrameAddress(_) => unreachable!(),
                            });
                        }

                        unsafe {
                            page_table.map_to(
                                last_page,
                                new_frame,
                                page_table_flags,
                                frame_allocator,
                            )?
                        }
                        .flush();
                    }

                    // Map additional frames.
                    let start_page: Page = Page::containing_address(VirtAddr::new(align_up(
                        zero_start.as_u64(),
                        Size4KiB::SIZE,
                    )));
                    let end_page = Page::containing_address(zero_end);
                    for page in Page::range_inclusive(start_page, end_page) {
                        let frame = frame_allocator
                            .allocate_frame()
                            .ok_or(MapToError::FrameAllocationFailed)?;
                        unsafe {
                            page_table.map_to(page, frame, page_table_flags, frame_allocator)?
                        }
                        .flush();
                    }

                    // zero
                    for offset in file_size..mem_size {
                        let addr = virt_start_addr + offset;
                        unsafe { addr.as_mut_ptr::<u8>().write(0) };
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}
