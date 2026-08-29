archs = ["x86_64"]

cuddld "test" {
  cmd  = [asm.hello]
  kind = "link-fail"
}

asm "hello" {
  source = "hello.S"
}
