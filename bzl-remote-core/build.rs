use std::error::Error;
use vergen::EmitBuilder;

fn main() -> Result<(), Box<dyn Error>> {
    // Generate the default 'cargo:' instruction output
    EmitBuilder::builder().all_build().all_git().emit()?;
    Ok(())
}
