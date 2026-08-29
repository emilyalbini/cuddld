archs = ["x86", "x86_64"]

cuddld "static" {
  cmd  = [c.hello_static, asm.syscall_write, asm.syscall_exit]
  kind = "run-pass"
}

cuddld "static_got" {
  cmd         = [c.hello_got, asm.syscall_write, asm.syscall_exit]
  kind        = "run-pass"
  debug-print = ["loaded-object=.text", "relocations-analysis", "relocated-object=.text,.got,@symbols"]
}

cuddld "pie_got" {
  cmd         = [c.hello_got, asm.syscall_write, asm.syscall_exit, "-pie"]
  kind        = "run-pass"
  debug-print = ["loaded-object=.text", "layout", "relocations-analysis", "relocated-object=.text,.got*,.interp,@symbols,@dynamic", "final-elf=.interp,.dyn*,.rel*,.*hash,@segments,@meta"]
}

cuddld "pie_got_relro" {
  cmd         = [c.hello_got, asm.syscall_write, asm.syscall_exit, "-pie", "-zrelro"]
  kind        = "run-pass"
  debug-print = ["layout", "final-elf=@segments,.gnu.hash"]
}

cuddld "pie_plt" {
  cmd         = [c.hello_plt, asm.syscall_write, asm.syscall_exit, "-pie"]
  kind        = "run-pass"
  debug-print = ["loaded-object=.text", "layout", "relocations-analysis", "relocated-object=.text,.got*,.plt,.interp,@symbols,@dynamic", "final-elf=.interp,.dyn*,.rel*,.*hash,@segments,@meta"]
}

cuddld "pie_plt_now" {
  cmd         = [c.hello_plt, asm.syscall_write, asm.syscall_exit, "-pie", "-znow"]
  kind        = "run-pass"
  debug-print = ["layout", "final-elf=.dynamic,@segments"]
}

c "hello_static" {
  source     = "hello.c"
  libc       = "freestanding"
  relocation = "static"
}

c "hello_got" {
  source     = "hello.c"
  libc       = "freestanding"
  relocation = "pic-only-got"
}

c "hello_plt" {
  source     = "hello.c"
  libc       = "freestanding"
  relocation = "pic"
}

asm "syscall_exit" {
  output = "syscall.exit.o"
  source = "../_shared/syscall.exit.${arch}.S"
}

asm "syscall_write" {
  output = "syscall.write.o"
  source = "../_shared/syscall.write.${arch}.S"
}
