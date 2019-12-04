#![no_std]
#![no_main]
#![warn(dead_code)]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};
use boot::{entry_point, BootInfo, MemoryRegionType};
use core::panic::PanicInfo;
use core::ptr::null_mut;
use kernel::allocator;
use kernel::libc::madvise;
use kernel::memory::{self, BootInfoFrameAllocator};
use kernel::syscall;
use kernel::{context_switch, exit_qemu, println, QemuExitCode, MAPPER};
use x86_64::{
    align_up,
    structures::paging::{
        mapper::{MapToError, UnmapError},
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

use xmas_elf::program::{self, ProgramHeader64};

entry_point!(kernel_main);

static mut FRAME_ALLOCATOR: Option<BootInfoFrameAllocator> = None;

extern "C" {
    static _app_start_addr: usize;
    static _app_size: usize;
}

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    println!("Hello World!!");

    kernel::init();

    println!("{:#?}", boot_info);

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);

    unsafe { MAPPER.replace(memory::init(phys_mem_offset)) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&mut boot_info.memory_map) };

    allocator::init_heap(unsafe { MAPPER.as_mut().unwrap() }, &mut frame_allocator)
        .expect("heap initialization failed");

    allocator::init_stack(unsafe { MAPPER.as_mut().unwrap() }, &mut frame_allocator)
        .expect("heap initialization failed");

    unsafe {
        FRAME_ALLOCATOR.replace(frame_allocator);
    }

    unsafe {
        context_switch(
            kernel_main_with_stack_protection,
            allocator::STACK_START + allocator::STACK_SIZE,
        )
    }
}

fn kernel_main_with_stack_protection() -> ! {
    let mut frame_allocator = unsafe { FRAME_ALLOCATOR.take().unwrap() };
    let mapper = unsafe { MAPPER.as_mut().unwrap() };

    frame_allocator.set_region_type_usable(MemoryRegionType::KernelStack);

    let ret = madvise(null_mut(), 0, 0);
    println!("madvise() = {:#?}", ret);

    // allocate a number on the stack
    let stack_value = 41;
    println!("stack_value at {:p}", (&stack_value) as *const i32);

    // allocate a number on the heap
    let heap_value = Box::new(41);
    println!("heap_value at {:p}", heap_value);

    // create a dynamically sized vector
    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }
    println!("vec at {:p}", vec.as_slice());

    // create a reference counted vector -> will be freed when count reaches 0
    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    println!(
        "current reference count is {}",
        Rc::strong_count(&cloned_reference)
    );
    core::mem::drop(reference_counted);
    println!(
        "reference count is {} now",
        Rc::strong_count(&cloned_reference)
    );

    #[cfg(test)]
    test_main();

    exec_app(mapper, &mut frame_allocator);

    println!("It did not crash!");
    exit_qemu(QemuExitCode::Success);
    kernel::hlt_loop()
}

pub const PML4_SIZE: usize = 0x0000_0080_0000_0000;
pub const PML4_MASK: usize = 0x0000_ff80_0000_0000;

/// Offset to user image
pub const USER_OFFSET: usize = 0;
pub const USER_PML4: usize = (USER_OFFSET & PML4_MASK) / PML4_SIZE;

/// Offset to user TCB
/// Each process has 4096 bytes, at an offset of 4096 * PID
pub const USER_TCB_OFFSET: usize = 0xB000_0000;

/// Offset to user arguments
pub const USER_ARG_OFFSET: usize = USER_OFFSET + PML4_SIZE / 2;

/// Offset to user heap
pub const USER_HEAP_OFFSET: usize = USER_OFFSET + PML4_SIZE;
pub const USER_HEAP_PML4: usize = (USER_HEAP_OFFSET & PML4_MASK) / PML4_SIZE;

/// Offset to user grants
pub const USER_GRANT_OFFSET: usize = USER_HEAP_OFFSET + PML4_SIZE;
pub const USER_GRANT_PML4: usize = (USER_GRANT_OFFSET & PML4_MASK) / PML4_SIZE;

/// Offset to user stack
pub const USER_STACK_OFFSET: usize = USER_GRANT_OFFSET + PML4_SIZE;
pub const USER_STACK_PML4: usize = (USER_STACK_OFFSET & PML4_MASK) / PML4_SIZE;
/// Size of user stack
//pub const USER_STACK_SIZE: usize = 1024 * 1024; // 1 MB
pub const USER_STACK_SIZE: usize = 4 * 1024; // 1 MB

/// Offset to user sigstack
pub const USER_SIGSTACK_OFFSET: usize = USER_STACK_OFFSET + PML4_SIZE;
pub const USER_SIGSTACK_PML4: usize = (USER_SIGSTACK_OFFSET & PML4_MASK) / PML4_SIZE;
/// Size of user sigstack
pub const USER_SIGSTACK_SIZE: usize = 256 * 1024; // 256 KB

fn exec_app(mapper: &mut OffsetPageTable, frame_allocator: &mut BootInfoFrameAllocator) {
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
                println!("{:#?}", segment);
            }
            ProgramHeader::Ph32(_) => panic!("does not support 32 bit elf files"),
        }
    }
    println!("app_entry_point={:#X}", entry_point);
    println!("USER_STACK_OFFSET={:#X}", USER_STACK_OFFSET);
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
    use x86_64::structures::paging::page::PageSize;

    let typ = segment.get_type().unwrap();
    match typ {
        program::Type::Load => {
            let mem_size = segment.mem_size;
            let file_size = segment.file_size;
            let file_offset = segment.offset;
            let phys_start_addr = file_start + file_offset;
            let virt_start_addr = VirtAddr::new(segment.virtual_addr);

            let start_page: Page = Page::containing_address(virt_start_addr);
            let start_frame = PhysFrame::containing_address(phys_start_addr);
            let end_frame = PhysFrame::containing_address(phys_start_addr + file_size - 1u64);

            let flags = segment.flags;
            let mut page_table_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
            if !flags.is_execute() {
                page_table_flags |= PageTableFlags::NO_EXECUTE
            };
            if flags.is_write() {
                page_table_flags |= PageTableFlags::WRITABLE
            };

            for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
                let offset = frame - start_frame;
                let page = start_page + offset;
                unsafe { page_table.map_to(page, frame, page_table_flags, frame_allocator)? }
                    .flush();
            }

            if mem_size > file_size {
                // .bss section (or similar), which needs to be zeroed
                let zero_start = virt_start_addr + file_size;
                let zero_end = virt_start_addr + mem_size;
                if zero_start.as_u64() & 0xfff != 0 {
                    // A part of the last mapped frame needs to be zeroed. This is
                    // not possible since it could already contains parts of the next
                    // segment. Thus, we need to copy it before zeroing.

                    // TODO: search for a free page dynamically
                    let temp_page: Page = Page::containing_address(VirtAddr::new(0xfeeefeee000));
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

                    let last_page = Page::containing_address(virt_start_addr + file_size - 1u64);
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
                    unsafe { page_table.map_to(page, frame, page_table_flags, frame_allocator)? }
                        .flush();
                }

                // zero
                for offset in file_size..mem_size {
                    let addr = virt_start_addr + offset;
                    unsafe { addr.as_mut_ptr::<u8>().write(0) };
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    exit_qemu(QemuExitCode::Failed);
    kernel::hlt_loop()
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::test_panic_handler(info)
}
