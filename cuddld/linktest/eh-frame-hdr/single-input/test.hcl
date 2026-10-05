archs = ["x86", "x86_64"]

cuddld "test" {
  cmd         = [rust.hello, "--eh-frame-hdr"]
  kind        = "link-pass"
  debug-print = ["loaded-object=.eh_*", "layout", "relocated-object=.eh_*", "final-elf=.eh_*"]
}

cuddld "gc" {
  cmd         = [rust.hello, "--eh-frame-hdr", "--gc-sections"]
  kind        = "link-pass"
  debug-print = ["final-elf=.eh_*"]
}

rust "hello" {
  source = "hello.rs"
}
