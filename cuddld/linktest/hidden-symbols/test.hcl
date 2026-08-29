archs = ["x86", "x86_64"]

cuddld "test" {
  cmd         = [asm.foo]
  kind        = "link-pass"
  debug-print = ["loaded-object=@symbols", "final-elf=.symtab"]
}

asm "foo" {
  source = "foo.S"
}
