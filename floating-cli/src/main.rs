use anyhow;
use floating_cli::process_arg;
use std::env::args;

fn main() -> anyhow::Result<()> {
    for arg in args().skip(1) {
        process_arg(&mut std::io::stdout(), &arg)?;
    }
    Ok(())
}
