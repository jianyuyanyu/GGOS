use crate::drivers::{console, serial};
use alloc::string::String;
use crossbeam_queue::ArrayQueue;
use pc_keyboard::DecodedKey;
use x86_64::instructions::interrupts;

once_mutex!(pub INPUT_BUF: ArrayQueue<DecodedKey>);

const DEFAULT_BUF_SIZE: usize = 128;

guard_access_fn!(pub get_input_buf(INPUT_BUF: ArrayQueue<DecodedKey>));

pub fn init() {
    init_INPUT_BUF(ArrayQueue::new(DEFAULT_BUF_SIZE));
    info!("Input Initialized.");
}

pub fn try_get_key() -> Option<DecodedKey> {
    interrupts::without_interrupts(|| {
        if let Some(key) = get_input_buf_for_sure().pop() {
            return Some(key);
        }
        None
    })
}

pub fn get_key() -> DecodedKey {
    loop {
        crate::utils::halt();
        interrupts::without_interrupts(|| {
            if let Some(k) = try_get_key() {
                return k;
            }
        })
    }
}

pub fn get_line() -> String {
    let mut s = String::with_capacity(DEFAULT_BUF_SIZE);
    loop {
        let key = get_key();
        if let DecodedKey::Unicode(k) = key {
            match k {
                '\n' => break,
                '\x08' => {
                    if !s.is_empty() {
                        console::backspace();
                        serial::backspace();
                        s.pop(); // remove previous char
                    }
                }
                c => {
                    print!("{}", k);
                    s.push(c)
                }
            }
        }
        console::get_console_for_sure().draw_hint();
    }
    println!();
    s
}
