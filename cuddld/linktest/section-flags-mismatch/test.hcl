archs = ["x86_64"]

cuddld "test" {
  cmd         = [asm.rx, asm.rwx]
  kind        = "link-pass"
  debug-print = ["layout"]
}

asm "rx" {
  source = "rx.S"
}

asm "rwx" {
  source = "rwx.S"
}
