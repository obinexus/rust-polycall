//! `cargo run --example demo -- [CONFIG]`: print the library version, describe
//! and strictly validate a configuration file through the real core.

use std::process::ExitCode;

fn main() -> ExitCode {
    let config = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "rust-polycallrc".to_owned());

    match run(&config) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rust-polycall demo: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(config: &str) -> Result<(), rust_polycall::Error> {
    rust_polycall::check_abi()?;
    println!(
        "rust-polycall using libpolycall {} (binding ABI {})",
        rust_polycall::version()?,
        rust_polycall::abi_version()
    );
    println!("{}", rust_polycall::describe(Some(config))?);
    rust_polycall::run_config(Some(config), true)?;
    println!("{config}: valid for running with this build");
    Ok(())
}
