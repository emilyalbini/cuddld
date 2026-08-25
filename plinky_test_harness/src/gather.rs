use crate::Step;
use crate::picohcl::ast::Statement;
use crate::picohcl::{HclDeserializer, parse_picohcl};
use crate::tests::{Arch, Test, TestStep};
use crate::utils::err_str;
use plinky_error::{ErasedContext as _, ErasedError, bail};
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use test::{ShouldPanic, TestDesc, TestDescAndFn, TestFn, TestName, TestType};
use toml::Value;

pub(crate) fn gather(
    path: &Path,
    prefix: &str,
    define_steps: DefineStepsFn,
) -> Result<Vec<TestDescAndFn>, ErasedError> {
    let mut tests = Vec::new();

    for entry in path.read_dir()? {
        let entry = entry?.path();

        let toml = entry.join("test.toml");
        let hcl = entry.join("test.hcl");
        if toml.is_file() && hcl.is_file() {
            bail!("both {toml:?} and {hcl:?} exist");
        } else if toml.is_file() {
            create_tests(&mut tests, prefix, Input::Toml(toml.clone()), define_steps)
                .with_context(|| format!("failed to create tests from {}", toml.display()))?;
        } else if hcl.is_file() {
            create_tests(&mut tests, prefix, Input::Hcl(hcl.clone()), define_steps)
                .with_context(|| format!("failed to create tests from {}", hcl.display()))?;
        } else if entry.is_dir() {
            let prefix = format!("{}{}/", prefix, entry.file_name().unwrap().to_str().unwrap());
            tests.extend(gather(&entry, &prefix, define_steps)?);
        }
    }

    Ok(tests)
}

fn create_tests(
    tests: &mut Vec<TestDescAndFn>,
    prefix: &str,
    input: Input,
    define_steps: DefineStepsFn,
) -> Result<(), ErasedError> {
    let source_dir = match &input {
        Input::Toml(path) | Input::Hcl(path) => path.parent().unwrap(),
    };
    let name = format!("{}{}", prefix, source_dir.file_name().unwrap().to_str().unwrap());

    let mut definers = Vec::new();
    match &input {
        Input::Toml(path) => {
            let raw = std::fs::read_to_string(path)?;
            let toml: Toml = toml::from_str(&raw)?;
            for &arch in &toml.archs {
                definers.push(DefineSteps {
                    arch,
                    ignore: toml.ignore.clone(),
                    undefined_toml: toml.steps.clone(),
                    undefined_hcl: BTreeMap::new(),
                    defined: Vec::new(),
                    defined_leafs: Vec::new(),
                })
            }
        }
        Input::Hcl(path) => {
            let raw = std::fs::read_to_string(path)?;
            let parsed = parse_picohcl(&raw)?;

            let mut de = HclDeserializer::from_statements(parsed.contents.clone());
            let archs: Vec<Arch> = de.field("archs")?;
            let ignore = de.opt_field("ignore")?;

            let mut undefined_hcl: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();
            for statement in de.remaining() {
                let block = match statement {
                    Statement::Assignment(assignment) => {
                        bail!("unsupported top-level key: {}", assignment.key);
                    }
                    Statement::Block(block) => block,
                };
                let Some(step_name) = block.name else {
                    bail!("step {} is missing a name", block.kind);
                };
                undefined_hcl
                    .entry(block.kind)
                    .or_default()
                    .insert(step_name, HclDeserializer::from_statements(block.contents));
            }

            for arch in archs {
                definers.push(DefineSteps {
                    arch,
                    ignore: ignore.clone(),
                    undefined_toml: BTreeMap::new(),
                    undefined_hcl: undefined_hcl.clone(),
                    defined: Vec::new(),
                    defined_leafs: Vec::new(),
                })
            }
        }
    }

    for mut definer in definers {
        let arch = definer.arch;
        define_steps(&mut definer)?;

        let missing_step_kinds = definer
            .undefined_toml
            .into_keys()
            .chain(definer.undefined_hcl.into_keys())
            .collect::<Vec<_>>();
        if !missing_step_kinds.is_empty() {
            bail!(
                "test contains the following undefined step types: {}",
                missing_step_kinds.join(", ")
            );
        }

        for leaf in &definer.defined_leafs {
            let mut steps = definer.defined.clone();
            steps.push(leaf.clone());

            let leaf_name = if definer.defined_leafs.len() > 1 {
                let name = leaf.name.split_once('.').expect("bad step name").1;
                format!("{name}, ")
            } else {
                String::new()
            };

            let test = Test { arch, steps, source_dir: source_dir.into() };

            tests.push(TestDescAndFn {
                desc: TestDesc {
                    name: TestName::DynTestName(format!("{name} ({leaf_name}{arch})")),
                    ignore: definer.ignore.is_some(),
                    ignore_message: definer.ignore.clone().map(leak),
                    source_file: "",
                    start_line: 0,
                    start_col: 0,
                    end_line: 0,
                    end_col: 0,
                    should_panic: ShouldPanic::No,
                    compile_fail: false,
                    no_run: false,
                    test_type: TestType::IntegrationTest,
                },
                testfn: TestFn::DynTestFn(Box::new(move || err_str(test.run()))),
            });
        }
    }

    Ok(())
}

#[derive(serde::Deserialize)]
struct Toml {
    archs: Vec<Arch>,
    #[serde(default)]
    ignore: Option<String>,
    #[serde(flatten)]
    steps: BTreeMap<String, BTreeMap<String, Value>>,
}

pub(crate) type DefineStepsFn = fn(&mut DefineSteps) -> Result<&mut DefineSteps, ErasedError>;

pub struct DefineSteps {
    arch: Arch,
    ignore: Option<String>,
    undefined_toml: BTreeMap<String, BTreeMap<String, Value>>,
    undefined_hcl: BTreeMap<String, BTreeMap<String, HclDeserializer>>,
    defined: Vec<TestStep>,
    defined_leafs: Vec<TestStep>,
}

impl DefineSteps {
    pub fn define_builtins(&mut self) -> Result<&mut Self, ErasedError> {
        self.define::<crate::steps::asm::AsmStep>("asm")?
            .define::<crate::steps::ld::LdStep>("ld")?
            .define::<crate::steps::c::CStep>("c")?
            .define::<crate::steps::rust::RustStep>("rust")?
            .define::<crate::steps::ar::ArStep>("ar")?
            .define::<crate::steps::rename::RenameStep>("rename")?
            .define::<crate::steps::dir::DirStep>("dir")
    }

    // Deserializing the Value into the concrete type cannot be done through dynamic dispatching.
    // This approach is similar to the one proposed in libcore for ErasedError's provider API, where this
    // method is invoked for every set of steps to process.
    pub fn define<S: Step + DeserializeOwned + FromHcl + 'static>(
        &mut self,
        kind_name: &str,
    ) -> Result<&mut Self, ErasedError> {
        if let Some(steps) = self.undefined_toml.remove(kind_name) {
            for (step_name, data) in steps {
                let name = format!("{kind_name}.{step_name}");
                let step = Box::new(
                    data.try_into::<S>().with_context(|| format!("failed to parse step {name}"))?,
                );
                if step.is_leaf() {
                    self.defined_leafs.push(TestStep::new(&name, step));
                } else {
                    self.defined.push(TestStep::new(&name, step));
                }
            }
        }
        if let Some(steps) = self.undefined_hcl.remove(kind_name) {
            for (step_name, mut de) in steps {
                let name = format!("{kind_name}.{step_name}");
                let step = Box::new(
                    S::from_hcl(&mut de).with_context(|| format!("failed to parse step {name}"))?,
                );
                de.ensure_exhaustive()?;
                if step.is_leaf() {
                    self.defined_leafs.push(TestStep::new(&name, step));
                } else {
                    self.defined.push(TestStep::new(&name, step));
                }
            }
        }
        Ok(self)
    }
}

pub trait FromHcl: Sized {
    fn from_hcl(de: &mut HclDeserializer) -> Result<Self, ErasedError>;
}

enum Input {
    Toml(PathBuf),
    Hcl(PathBuf),
}

fn leak(string: String) -> &'static str {
    Box::leak(Box::new(string)).as_str()
}
