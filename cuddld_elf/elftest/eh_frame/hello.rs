#![no_std]

#[no_mangle]
pub fn _start() -> ! {
    loop {}
}

#[panic_handler]
fn panic_handler(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
