use std::process::Command;

/// Sentinel environment variable used to distinguish wrapper invocations from normal xtask
/// invocations.
const ENV_KEY: &str = "XTASK_RUSTC_WRAPPER";

/// Handle a potential rustc wrapper invocation, then return.
///
/// Call this at the very **first** statement in your xtask `main` function. When Cargo is invoking
/// the xtask binary as a `RUSTC_WORKSPACE_WRAPPER`, this function runs `rustc -Awarnings
/// <origina-args>` and terminates the process. When the xtask is invoked normally by the
/// developer, this functions is a no-op and returns immediately, so the rest of `main` executes as
/// usual.
///
/// # Panics
///
/// Panics if the process is running as a rustc wrapper but the rustc path argument is missing or
/// if the rustc subprocess cannot be spawned.
///
/// # Example
///
/// rust
/// ```no_run
/// fn main() {
///     xtask_no_warnings::init();
///
///     // Your xtask logic starts here.
/// }
/// ```
#[allow(clippy::needless_doctest_main)]
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
        .unwrap_or_else(|e| panic!("failed to spawn rustc (`{}`): {e}", rustc.to_string_lossy()));
    std::process::exit(status.code().unwrap_or(1));
}

/// Configure an existing Cargo `Command` to suppress warnings in workspace members.
///
/// This sets two environment variables on the command:
///
/// - `RUSTC_WORKSPACE_WRAPPER` - points to the current xtask executable so that Cargo routes
///   workspace member compilation through it.
/// - `XTASK_RUSTC_WRAPPER - a sentinel that `init` uses to detect wrapper invocations.
///
/// Dependencies are **not** wrapped and their cached artifacts remain valid regardless of whether
/// you call this function.
///
/// # Panics
///
/// Panics if the path to the current executable cannot be determined.
///
/// # Example
///
/// rust
/// ```no_run
/// fn build(no_warnings: bool) {
///     let mut cmd = std::process::Command::new("cargo");
///     cmd.args(["build", "--release"]);
///
///     if no_warnings {
///         xtask_no_warnings::setup(&mut cmd);
///     }
///
///     cmd.status().expect("cargo failed");
/// }
/// ```
pub fn setup(cmd: &mut Command) {
    let wrapper =
        std::env::current_exe().expect("cannot determinate the path to the current executable");
    cmd.env("RUSTC_WORKSPACE_WRAPPER", wrapper)
        .env(ENV_KEY, "1");
}

/// Return a Cargo `Command` pre-configured to suppress warnings in workspace members.
///
/// This is a convenience wrapper around `setup`. The returned command already has
/// `RUSTC_WORKSPACE_WRAPPER` and `XTASK_RUSTC_WRAPPER` set, you only need to append subcommand and
/// flags.
///
/// The Cargo executable is taken from the `CARGO` environment variable when available (which Cargo
/// sets automatically), falling back to `cargo` if not set.
///
/// # Panics
///
/// Panics if the path to the current executable cannot be determined.
///
/// # Example
///
/// rust
/// ```no_run
/// fn build_without_warning() {
///     xtask_no_warnings::cargo_command()
///         .args(["build", "--release"])
///         .status()
///         .expect("cargo command failed");
/// }
/// ```
pub fn cargo_command() -> Command {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut cmd = Command::new(cargo);
    setup(&mut cmd);
    cmd
}
