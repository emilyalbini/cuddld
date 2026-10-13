archs = ["x86_64"]

cuddld "test" {
  cmd         = [asm.test, "-pie"]
  kind        = "link-pass"
  debug-print = ["loaded-object=*_array", "final-elf=*_array,.dynamic"]
}

asm "test" {
  source = "test.S"
}
