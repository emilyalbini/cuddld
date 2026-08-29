archs = ["x86", "x86_64"]

cuddld "test" {
  cmd         = [asm.test]
  kind        = "link-pass"
  debug-print = ["loaded-object=@symbols"]
}

asm "test" {
  source = "test.S"
}
