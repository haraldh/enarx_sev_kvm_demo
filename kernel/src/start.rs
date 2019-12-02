#![no_std]
#![warn(dead_code)]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use crate::allocator;
use crate::libc::madvise;
use crate::memory::{self, BootInfoFrameAllocator};
use crate::{context_switch, exit_qemu, println, QemuExitCode, MAPPER};
use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};
use boot::{entry_point, BootInfo, MemoryRegionType};
use core::panic::PanicInfo;
use core::ptr::null_mut;
use x86_64::VirtAddr;

entry_point!(kernel_main);

static mut FRAME_ALLOCATOR: Option<BootInfoFrameAllocator> = None;

extern "C" {
    static _app_start_addr: usize;
    static _app_size: usize;
}

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    println!("Hello World!!");

    crate::init();

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
    frame_allocator.set_region_type_usable(MemoryRegionType::KernelStack);
    unsafe {
        FRAME_ALLOCATOR.replace(frame_allocator);
    }

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

    use xmas_elf::program::ProgramHeader;

    // Extract required information from the ELF file.
    let mut segments = Vec::new();
    let entry_point;
    unsafe {
        println!("app start {:#X}", &_app_start_addr as *const _ as u64);
        println!("app size {:#X}", &_app_size as *const _ as u64);
    }
    {
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
                    let val = *header;
                    segments.push(val)
                }
                ProgramHeader::Ph32(_) => panic!("does not support 32 bit elf files"),
            }
        }
        println!("{:#?}", segments);
        println!("app_entry_point={:#X}", entry_point);
    }

    println!("It did not crash!");
    exit_qemu(QemuExitCode::Success);
    crate::hlt_loop()
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    exit_qemu(QemuExitCode::Failed);
    crate::hlt_loop()
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::test_panic_handler(info)
}

#[lang = "eh_personality"]
#[no_mangle]
pub extern "C" fn rust_eh_personality() {}
