archs = ["x86_64"]

cuddld "test" {
  cmd = [asm.foo, "--eh-frame-hdr"]
  kind        = "link-pass"
  debug-print = ["layout"]
}

asm "foo" {
  source = "foo.S"
}
