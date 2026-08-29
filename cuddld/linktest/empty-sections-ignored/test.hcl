archs = ["x86_64"]

cuddld "test" {
  cmd         = [asm.foo]
  kind        = "link-pass"
  debug-print = ["loaded-object"]
}

asm "foo" {
  source = "foo.S"
}
