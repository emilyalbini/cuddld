use plinky_elf::ElfReader;
use plinky_elf::writer::Writer;
use plinky_elf::writer::layout::Layout;
use plinky_error::{ErasedError, erased};
use plinky_test_harness::picohcl::{FromHcl, HclDeserializer};
use plinky_test_harness::{Step, TestContext};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
struct ReadElfStep {
    file: PathBuf,
    roundtrip: bool,
    filter: Option<String>,
}

impl Step for ReadElfStep {
    fn run(&self, ctx: TestContext<'_>) -> Result<(), ErasedError> {
        let file = ctx.maybe_relative_to_src(&self.file);
        self.read(&ctx, &file)?;

        if self.roundtrip {
            let roundtrip = self.roundtrip(&ctx, &file)?;
            self.read(&ctx, &roundtrip)?;
        }

        Ok::<(), ErasedError>(())
    }

    fn is_leaf() -> bool {
        true
    }
}

impl FromHcl for ReadElfStep {
    fn from_hcl(de: &mut HclDeserializer) -> Result<Self, ErasedError> {
        Ok(Self {
            file: de.field("file")?,
            roundtrip: de.opt_field("roundtrip")?.unwrap_or(true),
            filter: de.opt_field("filter")?,
        })
    }
}

impl ReadElfStep {
    fn read(&self, ctx: &TestContext<'_>, file: &Path) -> Result<(), ErasedError> {
        println!("reading {}...", file.display());

        let mut command = Command::new(env!("CARGO_BIN_EXE_read-elf"));
        command.arg(file);
        if let Some(filter) = &self.filter {
            command.arg(filter);
        }

        let mut runner = ctx.run_and_snapshot();
        let outcome = runner.run("reading ELF", &mut command)?;
        runner.persist();

        if !outcome {
            erased!("failed to read the ELF file");
        }

        Ok(())
    }

    fn roundtrip(&self, ctx: &TestContext<'_>, file: &Path) -> Result<PathBuf, ErasedError> {
        println!("writing the file back for the roundtrip...");

        let dest = ctx.dest.join(ctx.step_name).join("roundtrip").join(file.file_name().unwrap());
        std::fs::create_dir_all(dest.parent().unwrap())?;

        let object = ElfReader::new(&mut BufReader::new(File::open(file)?))?.into_object()?;
        Writer::new(
            &mut BufWriter::new(File::create_new(&dest)?),
            &object,
            Layout::new(&object, None)?,
        )?
        .write()?;

        Ok(dest)
    }
}

#[derive(Debug)]
struct ReadDynamicStep {
    file: PathBuf,
}

impl Step for ReadDynamicStep {
    fn run(&self, ctx: TestContext<'_>) -> Result<(), ErasedError> {
        let file = ctx.maybe_relative_to_src(&self.file);
        println!("reading {}...", file.display());

        let mut command = Command::new(env!("CARGO_BIN_EXE_read-dynamic"));
        command.arg(&file);

        let mut runner = ctx.run_and_snapshot();
        let outcome = runner.run("reading dynamic information", &mut command)?;
        runner.persist();

        if !outcome {
            erased!("failed to read the dynamic information in the ELF");
        }
        Ok(())
    }

    fn is_leaf() -> bool {
        true
    }
}

impl FromHcl for ReadDynamicStep {
    fn from_hcl(de: &mut HclDeserializer) -> Result<Self, ErasedError> {
        Ok(Self { file: de.field("file")? })
    }
}

fn main() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("elftest");
    plinky_test_harness::main(&path, |steps| {
        steps
            .define_builtins()?
            .define::<ReadElfStep>("read-elf")?
            .define::<ReadDynamicStep>("read-dynamic")
    });
}
