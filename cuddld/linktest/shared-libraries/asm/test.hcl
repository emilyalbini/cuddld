archs = ["x86", "x86_64"]

cuddld "test" {
  cmd         = [asm.foo, "-shared", "-o", "libfoo.so"]
  kind        = "link-pass"
  debug-print = ["final-elf=@meta,.dynamic,.dynsym"]
}

asm "foo" {
  source = "foo.S"
}
