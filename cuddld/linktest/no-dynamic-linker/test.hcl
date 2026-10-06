archs = ["x86_64"]

cuddld "test" {
  cmd         = [asm.test, "--no-dynamic-linker", "-pie"]
  kind        = "link-pass"
  debug-print = ["final-elf=@segments"]
}

asm "test" {
  source = "test.S"
}
