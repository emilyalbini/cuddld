archs = ["x86_64"]

cuddld "test" {
  cmd  = [asm.test]
  kind = "link-fail"
}

asm "test" {
  source = "test.S"
}
