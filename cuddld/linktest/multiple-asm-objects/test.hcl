archs = ["x86", "x86_64"]

cuddld "test" {
  cmd         = [asm.goodbye, asm.hello]
  kind        = "run-pass"
  debug-print = ["loaded-object", "relocated-object", "layout", "final-elf"]
}

asm "hello" {
  source = "hello.S"
}

asm "goodbye" {
  source = "goodbye.S"
}
