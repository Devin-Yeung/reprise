fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match xtask::run(&args) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("xtask: {err}");
            std::process::ExitCode::from(2)
        }
    }
}
