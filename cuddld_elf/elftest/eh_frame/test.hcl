archs = ["x86", "x86_64"]

read-elf "read" {
  file   = ld.eh_frame_hdr
  filter = ".eh_*"
}

ld "eh_frame_hdr" {
  output     = "a.out"
  content    = [rust.hello]
  extra-args = ["--eh-frame-hdr"]
}

rust "hello" {
  source     = "hello.rs"
}
