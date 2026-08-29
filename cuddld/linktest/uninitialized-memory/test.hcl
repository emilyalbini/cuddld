archs = ["x86_64"]

cuddld "test" {
  cmd         = [asm.bss]
  kind        = "run-pass"
  debug-print = ["loaded-object=.bss", "layout", "final-elf=.bss,@segments"]
}

asm "bss" {
  source = "bss.S"
}
