use std::process::Command;

const ENV_KEY: &str = "XTASK_RUSTC_WRAPPER";

pub fn init() {
    if std::env::var(ENV_KEY).is_ok() {
        return;
    }

    let mut args = std::env::args_os().skip(1);
    let rustc = args.next().expect("no rustc path was provided");

    let status = Command::new(&rustc)
        .arg("-Awarnings")
        .args(args)
        .status()
        .unwrap_or_else(|e| {
            panic!("failed to spawn rustc (`{}`): {e}",
            rustc.to_string_lossy()
        )
    });
    std::process::exit(status.code().unwrap_or(1));
}

pub fn setup(cmd: &mut Command) {
    let wrapper = std::env::current_exe().expect("cannot determinate the path to the current executable");
    cmd
        .env("RUSTC_WORKSPACE_WRAPPER", wrapper)
        .env(ENV_KEY, "1");
}

pub fn cargo_command() -> Command {
    let cargo = std::env::var("CARGO").unwrap_or_else(|| "cargo".into());
    let mut cmd = Command::new(cargo);
    setup(&mut cmd);
    cmd
}
