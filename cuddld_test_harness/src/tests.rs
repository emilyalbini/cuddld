use crate::Step;
use crate::picohcl::ast::ResolvedExpression;
use crate::picohcl::{FromHclString, HclContext};
use crate::utils::RunAndSnapshot;
use cuddld_error::{ErasedContext as _, ErasedError, bail};
use cuddld_utils::create_temp_dir;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) struct Test {
    pub(crate) arch: Arch,
    pub(crate) steps: Vec<TestStep>,
    pub(crate) source_dir: PathBuf,
}

impl Test {
    pub(crate) fn run(mut self) -> Result<(), ErasedError> {
        let mut ctx = HclContext::builtin();
        ctx.set_variable("arch", ResolvedExpression::String(self.arch.to_string()));

        // Cleanup for the temporary directory is done manually at the end, to ensure that the
        // build artifacts are present for inspection during a failure.
        let dest = create_temp_dir()?;
        eprintln!("output directory: {}", dest.display());

        loop {
            let mut progressed = false;
            let mut number_of_completed = 0;
            for step in &mut self.steps {
                match &mut step.stage {
                    StepStage::ToBeResolved(tbr) => {
                        if let Some(resolved) = tbr(&ctx)? {
                            step.stage = StepStage::Resolved(resolved);
                            progressed = true;
                        }
                    }
                    StepStage::Resolved(resolved) => {
                        resolved
                            .run(TestContext {
                                step_name: &step.name,
                                src: &self.source_dir,
                                dest: &dest,
                                arch: self.arch,
                                hcl: &mut ctx,
                            })
                            .with_context(|| format!("failed to execute step {}", step.name))?;
                        step.stage = StepStage::Complete;
                        progressed = true;
                    }
                    StepStage::Complete => {
                        number_of_completed += 1;
                    }
                }
            }

            if number_of_completed == self.steps.len() {
                // We are done!
                break;
            } else if !progressed {
                // There are either variables pointing to missing steps, or circular dependencies.
                let unmet_dependencies = self
                    .steps
                    .iter()
                    .filter(|s| !matches!(s.stage, StepStage::Complete))
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>();
                eprintln!("these steps have unmet dependencies: {}", unmet_dependencies.join(", "));
                bail!("made no progress");
            }
        }

        std::fs::remove_dir_all(&dest).context("failed to remove the temporary directory")?;
        Ok(())
    }
}

pub struct TestContext<'a> {
    pub step_name: &'a str,
    pub src: &'a Path,
    pub dest: &'a Path,
    pub arch: Arch,
    pub hcl: &'a mut HclContext,
}

impl TestContext<'_> {
    pub fn maybe_relative_to_src(&self, path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();
        if path.is_absolute() { path.into() } else { self.src.join(path) }
    }

    pub fn run_and_snapshot(&self) -> RunAndSnapshot {
        let name = self.step_name.split_once('.').expect("invalid step name").1.replace("_", "-");
        let arch = match self.arch {
            Arch::X86 => "32bit",
            Arch::X86_64 => "64bit",
        };

        RunAndSnapshot::new(&format!("{name}-{arch}"), self.src)
    }
}

#[derive(Debug)]
pub(crate) struct TestStep {
    pub(crate) name: String,
    stage: StepStage,
}

impl TestStep {
    pub(crate) fn new(name: &str, resolve: ResolveStepFn) -> Self {
        Self { name: name.into(), stage: StepStage::ToBeResolved(resolve) }
    }
}

enum StepStage {
    ToBeResolved(ResolveStepFn),
    Resolved(Box<dyn Step>),
    Complete,
}

impl std::fmt::Debug for StepStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ToBeResolved(_) => write!(f, "ToBeResolved(...)"),
            Self::Resolved(step) => f.debug_tuple("Resolved").field(step).finish(),
            Self::Complete => write!(f, "Complete"),
        }
    }
}

type ResolveStepFn =
    Box<dyn FnMut(&HclContext) -> Result<Option<Box<dyn Step>>, ErasedError> + Send>;

#[derive(Debug, Clone, Copy)]
pub enum Arch {
    X86,
    X86_64,
}

impl std::fmt::Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Arch::X86 => f.write_str("x86"),
            Arch::X86_64 => f.write_str("x86_64"),
        }
    }
}

impl FromHclString for Arch {
    fn from_string(input: String) -> Result<Self, ErasedError> {
        Ok(match input.as_str() {
            "x86" => Arch::X86,
            "x86_64" => Arch::X86_64,
            other => bail!("unknown arch: {other}"),
        })
    }
}
