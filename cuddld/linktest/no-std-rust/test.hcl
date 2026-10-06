archs = ["x86_64"]

cuddld-rustc "rustc" {
  cmd  = ["-Cpanic=abort", to_path("hello.rs")]
  kind = "run-pass"
}

cuddld "test" {
  cmd  = [rust.hello]
  kind = "run-pass"
}

rust "hello" {
  source = "hello.rs"
}
