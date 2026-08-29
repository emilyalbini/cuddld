archs = ["x86_64"]

cuddld "test" {
  cmd  = [ar.foo]
  kind = "link-fail"
}

ar "foo" {
  output       = "archive.a"
  content      = [asm.foo]
  symbol-table = false
}

asm "foo" {
  source = "foo.S"
}
