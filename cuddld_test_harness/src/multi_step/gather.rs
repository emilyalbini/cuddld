use crate::multi_step::Step;
use crate::multi_step::tests::{Arch, Test, TestStep};
use crate::nextest::Test as NextestTest;
use crate::picohcl::ast::Statement;
use crate::picohcl::{FromHcl, HclContext, HclDeserializer, HclResolver, parse_picohcl};
use cuddld_error::{ErasedContext as _, ErasedError, bail, erased};
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) fn gather(
    path: &Path,
    prefix: &str,
    define_steps: DefineStepsFn,
) -> Result<Vec<NextestTest>, ErasedError> {
    let mut tests = Vec::new();

    for entry in path.read_dir()? {
        let entry = entry?.path();

        let toml = entry.join("test.toml");
        let hcl = entry.join("test.hcl");
        if toml.is_file() {
            bail!("toml tests are no longer supported: {toml:?}");
        } else if hcl.is_file() {
            let result = create_tests(&mut tests, prefix, &hcl, define_steps)
                .with_context(|| format!("failed to create tests from {}", hcl.display()));
            match result {
                Ok(()) => {}
                Err(err) => create_failing_test(&mut tests, &hcl, err),
            }
        } else if entry.is_dir() {
            let prefix = format!("{}{}/", prefix, entry.file_name().unwrap().to_str().unwrap());
            tests.extend(gather(&entry, &prefix, define_steps)?);
        }
    }

    Ok(tests)
}

fn create_tests(
    tests: &mut Vec<NextestTest>,
    prefix: &str,
    path: &Path,
    define_steps: DefineStepsFn,
) -> Result<(), ErasedError> {
    let source_dir = path.parent().unwrap();
    let name = format!("{}{}", prefix, source_dir.file_name().unwrap().to_str().unwrap());

    let hcl = parse_hcl(path).with_context(|| format!("failed to parse {}", path.display()))?;
    for arch in hcl.archs {
        // We first define the steps to figure out what the leafs are and to do validation.
        let mut definer = DefineSteps {
            undefined: hcl.steps.clone(),
            defined: Vec::new(),
            defined_leafs: Vec::new(),
        };
        define_steps(&mut definer)?;

        let missing_step_kinds = definer.undefined.into_keys().collect::<Vec<_>>();
        if !missing_step_kinds.is_empty() {
            bail!(
                "test contains the following undefined step types: {}",
                missing_step_kinds.join(", ")
            );
        }

        for leaf in definer.defined_leafs {
            let leaf_name = leaf.name.clone();

            // Then, as unfortunately we cannot clone test steps, we re-define them and only extract
            // the steps we care about out of it.
            let mut definer = DefineSteps {
                undefined: hcl.steps.clone(),
                defined: Vec::new(),
                defined_leafs: Vec::new(),
            };
            define_steps(&mut definer)?;

            let mut steps = definer.defined;
            steps.push(leaf);

            let leaf_name = if definer.defined_leafs.len() > 1 {
                let name = leaf_name.split_once('.').expect("bad step name").1;
                format!("{name}, ")
            } else {
                String::new()
            };

            let test = Test { arch, steps, source_dir: source_dir.into() };

            tests.push(NextestTest {
                name: format!("{name} ({leaf_name}{arch})"),
                ignored: hcl.ignore.is_some(),
                exec: Box::new(move || test.run()),
            });
        }
    }

    Ok(())
}

fn create_failing_test(tests: &mut Vec<NextestTest>, file: &Path, err: ErasedError) {
    tests.push(NextestTest {
        name: file.to_string_lossy().to_string(),
        ignored: false,
        exec: Box::new(move || {
            eprintln!("{}", err.format_chain());
            panic!("test file failed to initialize");
        }),
    })
}

struct ParsedHcl {
    archs: Vec<Arch>,
    ignore: Option<String>,
    steps: BTreeMap<String, BTreeMap<String, HclResolver>>,
}

fn parse_hcl(path: &Path) -> Result<ParsedHcl, ErasedError> {
    let raw = std::fs::read_to_string(path)?;
    let parsed = parse_picohcl(&raw)?;

    // Top-level assignments shouldn't rely on anything non-builtin, so they can be resolved
    // immediately. Steps should be collected for later resolution.
    let mut assignments = Vec::new();
    let mut steps: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();
    for statement in parsed.contents {
        match statement {
            Statement::Assignment(_) => assignments.push(statement),
            Statement::Block(block) => {
                let Some(name) = block.name else {
                    bail!("block {} doesn't have a name", block.kind);
                };
                steps.entry(block.kind).or_default().insert(name, HclResolver::new(block.contents));
            }
        }
    }
    let mut assignments = HclResolver::new(assignments);
    assignments.try_resolve(&HclContext::builtin())?;
    let mut assignments = HclDeserializer::from_statements(
        assignments
            .resolved()
            .ok_or_else(|| erased!("not all top-level assignments could be resolved"))?,
    );

    let parsed = ParsedHcl {
        archs: assignments.field("archs")?,
        ignore: assignments.opt_field("ignore")?,
        steps,
    };
    assignments.ensure_exhaustive()?;
    Ok(parsed)
}

pub(crate) type DefineStepsFn = fn(&mut DefineSteps) -> Result<&mut DefineSteps, ErasedError>;

pub struct DefineSteps {
    undefined: BTreeMap<String, BTreeMap<String, HclResolver>>,
    defined: Vec<TestStep>,
    defined_leafs: Vec<TestStep>,
}

impl DefineSteps {
    pub fn define_builtins(&mut self) -> Result<&mut Self, ErasedError> {
        self.define::<crate::multi_step::steps::asm::AsmStep>("asm")?
            .define::<crate::multi_step::steps::ld::LdStep>("ld")?
            .define::<crate::multi_step::steps::c::CStep>("c")?
            .define::<crate::multi_step::steps::rust::RustStep>("rust")?
            .define::<crate::multi_step::steps::ar::ArStep>("ar")?
            .define::<crate::multi_step::steps::rename::RenameStep>("rename")?
            .define::<crate::multi_step::steps::dir::DirStep>("dir")
    }

    // Deserializing the Value into the concrete type cannot be done through dynamic dispatching.
    // This approach is similar to the one proposed in libcore for ErasedError's provider API, where this
    // method is invoked for every set of steps to process.
    pub fn define<S: Step + FromHcl + 'static>(
        &mut self,
        kind_name: &str,
    ) -> Result<&mut Self, ErasedError> {
        if let Some(steps) = self.undefined.remove(kind_name) {
            for (step_name, mut resolver) in steps {
                let name = format!("{kind_name}.{step_name}");

                let name_clone = name.clone();
                let step = Box::new(
                    move |ctx: &HclContext| -> Result<Option<Box<dyn Step>>, ErasedError> {
                        resolver.try_resolve(ctx)?;
                        if let Some(resolved) = resolver.resolved() {
                            let mut de = HclDeserializer::from_statements(resolved);
                            let step =
                                Box::new(S::from_hcl(&mut de).with_context(|| {
                                    format!("failed to parse step {name_clone}")
                                })?);
                            de.ensure_exhaustive().with_context(|| {
                                format!("step {name_clone} contains unknown fields")
                            })?;
                            Ok(Some(step))
                        } else {
                            Ok(None)
                        }
                    },
                );
                if S::is_leaf() {
                    self.defined_leafs.push(TestStep::new(&name, step));
                } else {
                    self.defined.push(TestStep::new(&name, step));
                }
            }
        }
        Ok(self)
    }
}
