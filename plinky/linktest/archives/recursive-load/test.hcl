archs = ["x86", "x86_64"]

plinky "test" {
  cmd         = [ar.archive]
  kind        = "link-pass"
  debug-print = ["loaded-object=.text"]
}

ar "archive" {
  output  = "archive.a"
  content = [asm.first, asm.second, asm.third]
}

asm "first" {
  source = "first.S"
}

asm "second" {
  source = "second.S"
}

asm "third" {
  source = "third.S"
}
