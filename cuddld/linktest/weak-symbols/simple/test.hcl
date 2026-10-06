archs = ["x86_64"]

cuddld "none" {
  cmd         = [asm.entry]
  kind        = "link-fail"
  debug-print = ["loaded-object=@symbols"]
}

cuddld "weak" {
  cmd         = [asm.entry, asm.weak]
  kind        = "link-pass"
  debug-print = ["loaded-object=@symbols"]
}

cuddld "weak_weak" {
  cmd         = [asm.entry, asm.weak, asm.weak]
  kind        = "link-pass"
  debug-print = ["loaded-object=@symbols"]
}

cuddld "weak_global" {
  cmd         = [asm.entry, asm.weak, asm.global]
  kind        = "link-pass"
  debug-print = ["loaded-object=@symbols"]
}

cuddld "global_weak" {
  cmd         = [asm.entry, asm.global, asm.weak]
  kind        = "link-pass"
  debug-print = ["loaded-object=@symbols"]
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
