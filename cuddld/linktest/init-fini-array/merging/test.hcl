archs = ["x86_64"]

cuddld "test" {
  cmd         = [asm.test]
  kind        = "link-pass"
  debug-print = ["loaded-object=.init_array*,.fini_array*", "final-elf=.init_array*,.fini_array*"]
}

asm "test" {
  source = "test.S"
}
