archs = ["x86_64"]

cuddld "weak_only" {
  cmd         = [asm.entry, ar.weak_only]
  kind        = "link-fail"
  debug-print = ["loaded-object=@inputs"]
}

cuddld "global_only" {
  cmd         = [asm.entry, ar.global_only]
  kind        = "link-pass"
  debug-print = ["loaded-object=@inputs"]
}

ar "weak_only" {
  output  = "libweak.a"
  content = [asm.weak]
}

ar "global_only" {
  output  = "libglobal.a"
  content = [asm.global]
}

asm "global" {
  source = "global.S"
}

asm "weak" {
  source = "weak.S"
}

asm "entry" {
  source = "entry.S"
}
