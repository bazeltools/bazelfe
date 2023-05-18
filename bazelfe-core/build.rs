use vergen::EmitBuilder;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Generate the default 'cargo:' instruction output
    EmitBuilder::builder().emit()?;
    Ok(())
}

