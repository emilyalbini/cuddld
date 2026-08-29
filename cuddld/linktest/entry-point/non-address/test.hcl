archs = ["x86_64"]

cuddld "test" {
  cmd  = [asm.code]
  kind = "link-fail"
}

asm "code" {
  source = "code.S"
}
