archs = ["x86_64"]

cuddld "missing_dir" {
  cmd  = [asm.hello]
  kind = "link-fail"
  link-env {
    PKG_CONFIG_PATH = "/dev/null"
  }
}

cuddld "parse_error" {
  cmd  = [asm.hello]
  kind = "link-fail"
  link-env {
    PKG_CONFIG_PATH = dir.pkg_config_mean
  }
}

cuddld "cli_error" {
  cmd  = [asm.hello]
  kind = "link-fail"
  link-env {
    PKG_CONFIG_PATH = dir.pkg_config_badflags
  }
}

dir "pkg_config_mean" {
  files = ["libmean.pc"]
}

dir "pkg_config_badflags" {
  files = ["libbadflags.pc"]
}

asm "hello" {
  source = "hello.S"
}
