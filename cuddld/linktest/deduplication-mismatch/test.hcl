archs = ["x86_64"]

cuddld "test" {
  cmd  = [asm.foo, asm.bar]
  kind = "link-pass"
}

asm "foo" {
  source = "foo.S"
}

asm "bar" {
  source = "bar.S"
}
