#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(core_intrinsics)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate bitflags;
#[macro_use]
mod utils;
#[macro_use]
mod output;
mod gdt;
mod interrupts;
mod memory;

use output::*;
use boot::BootInfo;
// use core::arch::asm;

boot::entry_point!(kernal_main);

pub fn kernal_main(boot_info: &'static BootInfo) -> ! {
    gdt::init();

    let graphic_info = &boot_info.graphic_info;
    display::initialize(graphic_info);

    display::get_display_for_sure().clear(Some(utils::colors::BACKGROUND), 0);

    console::initialize();
    println!("[+] Console Initialized.");

    logger::initialize();
    info!("Logger Initialized.");

    unsafe {
        interrupts::init();
    }
    info!("Interrupts Initialized.");

    trace!("Trace?");
    debug!("Debug Test.");
    warn!("Warning Test.");
    error!("ERROR!!!");

    for i in 0..10 {
        print!("[>] Waiting [{:>2}] ", i);
        for _ in 0..50 {
            for _ in 0..200_0000 {
                unsafe {
                    core::arch::asm!("nop");
                }
            }
            print!(">");
        }
        println!();
    }

    x86_64::instructions::interrupts::enable();
    info!("Interrupts Enabled.");

    loop {}
}
