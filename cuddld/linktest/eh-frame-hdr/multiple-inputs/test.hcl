archs = ["x86", "x86_64"]

cuddld "test" {
  cmd         = [rust.hello, c.hello, "--eh-frame-hdr"]
  kind        = "link-pass"
  debug-print = ["loaded-object=.eh_*", "layout", "relocated-object=.eh_*", "final-elf=.eh_*"]
}

c "hello" {
  source     = "hello.c"
  relocation = "static"
  libc       = "freestanding"
}

rust "hello" {
  source = "hello.rs"
}
