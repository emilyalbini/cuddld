use cuddld_diagnostics::widgets::Widget;
use cuddld_elf::ElfReader;
use cuddld_elf::render_elf::RenderElfFilters;
use cuddld_error::ErasedError;
use std::fs::File;

fn actual_main(args: &[String]) -> Result<(), ErasedError> {
    let (path, filters) = match args {
        [path] => (path, RenderElfFilters::all()),
        [path, filters] => (path, RenderElfFilters::parse(filters)?),
        _ => usage(),
    };

    let mut file = File::open(path)?;
    let object = ElfReader::new(&mut file)?.into_object()?;

    println!("{}", cuddld_elf::render_elf::render(&object, &filters).render_to_string());

    Ok(())
}

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if let Err(err) = actual_main(&args) {
        eprintln!("{}", err.format_chain());
        std::process::exit(1);
    }
}

fn usage() -> ! {
    eprintln!("usage: read-elf <path>");
    std::process::exit(1);
}
