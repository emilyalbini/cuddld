archs = ["x86"]

cuddld "test" {
  cmd         = [asm.stack, "-z", "execstack"]
  kind        = "run-pass"
  debug-print = ["final-elf=@segments"]
}

asm "stack" {
  source = "stack.S"
}
