use std::process::ExitCode;

fn main() -> ExitCode {
    let config = std::env::args().nth(1);

    match run(config.as_deref()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(u8::try_from(error.code).unwrap_or(1))
        }
    }
}

fn run(config: Option<&str>) -> Result<(), rust_polycall::PolycallError> {
    println!(
        "rust-polycall using libpolycall {}",
        rust_polycall::version()?
    );
    println!("{}", rust_polycall::describe(config)?);
    rust_polycall::run_config(config, true)
}
