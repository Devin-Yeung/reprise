use clap::Parser;

fn main() -> std::process::ExitCode {
    let cli = xtask::Cli::parse();
    match xtask::run(cli) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("xtask: {err:#}");
            std::process::ExitCode::from(2)
        }
    }
}
