archs = ["x86", "x86_64"]

cuddld "test" {
  cmd         = [asm.hello, "-e", "custom_entry"]
  kind        = "run-pass"
  debug-print = ["final-elf=@meta"]
}

asm "hello" {
  source = "hello.S"
}
