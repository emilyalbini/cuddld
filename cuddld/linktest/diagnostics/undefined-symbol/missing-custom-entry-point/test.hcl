archs = ["x86_64"]

cuddld "test" {
  cmd  = [asm.hello, "-e", "custom_entrypoint"]
  kind = "link-fail"
}

asm "hello" {
  source = "hello.S"
}
