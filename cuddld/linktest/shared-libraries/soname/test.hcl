archs = ["x86", "x86_64"]

cuddld "test" {
  cmd         = ["-shared", asm.foo, "-soname", "libfoo.so"]
  kind        = "link-pass"
  debug-print = ["relocated-object=.dynstr", "final-elf=.dynamic,.dynstr"]
}

asm "foo" {
  source = "foo.S"
}
