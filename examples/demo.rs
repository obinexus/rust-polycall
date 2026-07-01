// rust-polycall demo.
// Build the core first (../../setup.sh), then:  cargo run --example demo -- ../rust-polycallrc
use std::process::exit;

fn main() {
    let cfg = std::env::args().nth(1);
    println!("rust-polycall using libpolycall {}", rust_polycall::version());

    match rust_polycall::describe(cfg.as_deref()) {
        Ok(summary) => println!("{summary}"),
        Err(e) => {
            eprintln!("{e}");
            exit(e.code);
        }
    }

    if let Err(e) = rust_polycall::run_config(cfg.as_deref(), true) {
        eprintln!("{e}");
        exit(e.code);
    }
}
