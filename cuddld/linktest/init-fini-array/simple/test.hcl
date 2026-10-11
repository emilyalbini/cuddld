archs = ["x86_64"]

cuddld "test" {
  cmd         = [asm.test]
  kind        = "link-pass"
  debug-print = ["loaded-object=*_array", "final-elf=*_array"]
}

asm "test" {
  source = "test.S"
}
