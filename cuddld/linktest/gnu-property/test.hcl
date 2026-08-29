archs = ["x86", "x86_64"]

cuddld "isa_features2_one_file" {
  cmd         = [asm.entry_isa_features2]
  debug-print = ["loaded-object=@inputs", "final-elf=.note.gnu.property"]
  kind        = "link-pass"
}

cuddld "isa_features2_two_files" {
  cmd         = [asm.entry_isa_features2, asm.isa_features2]
  debug-print = ["loaded-object=@inputs", "final-elf=.note.gnu.property"]
  kind        = "link-pass"
}

cuddld "isa_two_files_with_features2_in_one" {
  cmd         = [asm.entry_isa_features2, asm.isa]
  debug-print = ["loaded-object=@inputs", "final-elf=.note.gnu.property"]
  kind        = "link-pass"
}

cuddld "duplicate_features2_used" {
  cmd  = [asm.duplicate_features2_used]
  kind = "link-fail"
}

cuddld "duplicate_isa_used" {
  cmd  = [asm.duplicate_isa_used]
  kind = "link-fail"
}

asm "duplicate_features2_used" {
  source          = "duplicate_features2_used.S"
  auxiliary-files = ["shared.S"]
  emit-x86-used   = false
}

asm "duplicate_isa_used" {
  source          = "duplicate_isa_used.S"
  auxiliary-files = ["shared.S"]
  emit-x86-used   = false
}

asm "entry_isa_features2" {
  source          = "entry_isa_features2.S"
  auxiliary-files = ["shared.S"]
  emit-x86-used   = false
}

asm "isa_features2" {
  source          = "isa_features2.S"
  auxiliary-files = ["shared.S"]
  emit-x86-used   = false
}

asm "isa" {
  source          = "isa.S"
  auxiliary-files = ["shared.S"]
  emit-x86-used   = false
}

asm "empty" {
  source        = "empty.S"
  emit-x86-used = false
}
