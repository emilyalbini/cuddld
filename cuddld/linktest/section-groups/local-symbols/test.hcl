archs = ["x86_64"]

cuddld "test" {
  cmd         = [asm.foo, asm.bar]
  kind        = "link-fail"
  debug-print = ["loaded-object=@symbols"]
}

asm "foo" {
  source = "foo.S"
}

asm "bar" {
  source = "bar.S"
}
