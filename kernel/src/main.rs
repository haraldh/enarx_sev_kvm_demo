#![no_std]
#![no_main]
#![warn(dead_code)]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};
use boot::{entry_point, BootInfo};
use core::panic::PanicInfo;
use core::ptr::null_mut;
use kernel::libc::madvise;
use kernel::{context_switch, exit_qemu, println, QemuExitCode, BOOTINFO, MAPPER};
use x86_64::VirtAddr;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use kernel::allocator;
    use kernel::memory::{self, BootInfoFrameAllocator};
    println!("Hello World!");

    kernel::init();

    println!("{:#?}", boot_info);

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    unsafe { MAPPER.replace(memory::init(phys_mem_offset)) };

    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(unsafe { MAPPER.as_mut().unwrap() }, &mut frame_allocator)
        .expect("heap initialization failed");

    allocator::init_stack(unsafe { MAPPER.as_mut().unwrap() }, &mut frame_allocator)
        .expect("heap initialization failed");

    unsafe {
        BOOTINFO.replace(boot_info);
    }

    unsafe {
        context_switch(
            kernel_main_with_stack_protection,
            allocator::STACK_START + allocator::STACK_SIZE,
        )
    }
}

fn kernel_main_with_stack_protection() -> ! {
    let ret = madvise(null_mut(), 0, 0);
    println!("madvise() = {:#?}", ret);

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

    #[cfg(commented_out)]
    {
        use xmas_elf::program::ProgramHeader;

        #[derive(Debug)]
        struct PhysOffset {
            offset: VirtAddr,
        }

        impl PhysOffset {
            fn phys_to_virt(&self, phys: PhysAddr) -> VirtAddr {
                let virt = self.offset.as_u64() + phys.as_u64();
                VirtAddr::new(virt)
            }
        }

        let phys_off = PhysOffset {
            offset: VirtAddr::new(boot_info.physical_memory_offset),
        };

        let mut app_region: Option<&MemoryRegion> = None;

        for region in boot_info.memory_map.iter() {
            if region.region_type == MemoryRegionType::App {
                app_region = Some(region);
                break;
            }
        }

        if let Some(app_region) = app_region {
            let app_start_ptr = phys_off.phys_to_virt(PhysAddr::new(app_region.range.start_addr()));
            let app_size = app_region.range.end_addr() - app_region.range.start_addr();

            // Extract required information from the ELF file.
            let mut segments = Vec::new();
            let entry_point;
            {
                let kernel =
                    unsafe { slice::from_raw_parts(app_start_ptr.as_ptr(), app_size as usize) };
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
                println!("entry_point={:#?}", entry_point);
            }
        }
    }

    println!("It did not crash!");
    exit_qemu(QemuExitCode::Success);
    kernel::hlt_loop()
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
