#![no_std]
#![no_main]

#[cfg(target_arch = "x86")]
fn write(fd: u64, content: &str) {
    let content = content.as_bytes();
    unsafe {
        core::arch::asm!(
            "push ebx",
            "mov eax, 4",
            "mov ebx, {fd}",
            "mov ecx, {ptr}",
            "mov edx, {len}",
            "int 0x80",
            "pop ebx",
            fd = in(reg) fd,
            ptr = in(reg) content.as_ptr(),
            len = in(reg) content.len(),
            out("eax") _,
            // ebx is used by llvm, so we have to push and pop it rather than clobbering it.
            out("ecx") _,
            out("edx") _
        );
    }
}

#[cfg(target_arch = "x86_64")]
fn write(fd: u64, content: &str) {
    let content = content.as_bytes();
    unsafe {
        core::arch::asm!(
            "mov rax, 1",
            "mov rdi, {fd}",
            "mov rsi, {ptr}",
            "mov rdx, {len}",
            "syscall",
            fd = in(reg) fd,
            ptr = in(reg) content.as_ptr(),
            len = in(reg) content.len(),
            out("rax") _,
            out("rcx") _,
            out("r11") _
        );
    }
}

#[cfg(target_arch = "x86")]
fn exit(code: u32) -> ! {
    unsafe {
        core::arch::asm!(
            "mov eax, 1",
            "mov ebx, {code}",
            "int 0x80",
            code = in(reg) code,
            options(noreturn)
        );
    }
}

#[cfg(target_arch = "x86_64")]
fn exit(code: u64) -> ! {
    unsafe {
        core::arch::asm!(
            "mov rax, 60",
            "mov rdi, {code}",
            "syscall",
            code = in(reg) code,
            options(noreturn)
        );
    }
}

#[no_mangle]
pub fn _start() -> ! {
    write(1, "Hello world\n");
    exit(0);
}

#[panic_handler]
fn panic_handler(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
