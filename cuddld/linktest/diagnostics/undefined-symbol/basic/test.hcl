archs = ["x86_64"]

cuddld "test" {
  cmd  = [c.test]
  kind = "link-fail"
}

c "test" {
  source     = "test.c"
  libc       = "freestanding"
  relocation = "static"
}
