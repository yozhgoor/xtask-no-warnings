#![allow(clippy::needless_doctest_main)]

//! Silence warnings in [xtask][xtask] builds without invalidating the dependency cache.
//!
//! # Purpose
//!
//! This is a micro crate with zero dependencies for use with xtask during development.
//!
//! The standard way to silence compiler warnings during development is to set
//! `RUSTFLAGS=-Awarnings`. It works, but it has a painful side effect: `RUSTFLAGS` is part of the
//! compiler fingerprint for **every** crate in the build graph. Toggling it forces Cargo to
//! recompile the entire project from scratch, including all dependencies. On machines with limited
//! resources (e.g. low-specs laptops, handheld devices, ...) this means minutes of wasted build
//! time every single time you flip the flag.
//!
//! This crate solves the problem by using [`RUSTC_WORKSPACE_WRAPPER`][workspace_wrapper] instead.
//! `RUSTC_WORKSPACE_WRAPPER` routes `rustc` invocations through a wrapper binary but **only for
//! workspace members**. Dependencies are compiled by `rustc` directly and their cached artifacts
//! remain valid regardless of whether the wrapper is active.
//!
//! The wrapper here is the xtask binary itself. At startup, [`init`] checks for a sentinel
//! environment variable. When Cargo invokes the xtask as a rustc wrapper, `init` forwards all
//! arguments to the real `rustc` with `-Awarnings` prepended and then exits. When the developer
//! invokes the xtask normally, `init` is a no-op and the rest of the `main` function runs as
//! usual.
//!
//! Because `RUSTC_WORKSPACE_WRAPPER` produces artifacts under a **separate fingerprint** from a
//! plain `rustc` run, the two modes (warning on or off) maintain independent caches for workspace
//! members. The very first toggle in each direction recompiles those crates, every subsequent
//! toggle hits the cache immediately.
//!
//! # Usage
//!
//! ## 1. Add the dependency to your xtask
//!
//! `xtask/Cargo.toml`
//! ```toml
//! [dependencies]
//! xtask-no-warnings = "0.1"
//! ```
//!
//! ## 2. Call init at the top of main
//!
//! `xtask/src/main.rs`
//! ```rust,no_run
//! fn main() {
//!     xtask_no_warnings::init();
//!
//!     // Your xtask logic here.
//! }
//! ```
//!
//! `init` must be the very first statement so that when Cargo invokes the xtask as a rustc
//! wrapper, it exits immediately before any of your setup code runs.
//!
//! ## 3. Spawn Cargo with or without warnings
//!
//! ### Option A - `setup`
//!
//! This function configures the current process to act as a workspace wrapper. Useful when you are
//! building the `Command` yourself and only want to add the wrapper conditionally.
//!
//! ```rust,no_run
//! fn build(no_warnings: bool) {
//!     let mut cmd = std::process::Command::new("cargo");
//!     cmd.args(["build", "--release"]);
//!
//!     if no_warnings {
//!         xtask_no_warnings::setup();
//!     }
//!
//!     cmd.status().expect("cargo failed");
//! }
//! ```
//!
//! ### Option B - `cargo_command`
//!
//! This function returns a `Command` for Cargo with the wrapper environment variable already set.
//! Append your subcommand and flags before running it.
//!
//! ```rust,no_run
//! fn build(no_warnings: bool) {
//!     let mut cmd = if no_warnings {
//!         xtask_no_warnings::cargo_command()
//!     } else {
//!         std::process::Command::new(std::env::var_os("CARGO").unwrap_or("cargo".into()))
//!     };
//!
//!     cmd.args(["build", "--release"])
//!         .status()
//!         .expect("cargo failed");
//! }
//! ```
//!
//! ## Basic xtask setup
//!
//! A typical project using a xtask workspace member looks like this:
//! ```toml
//! my-project/
//!   Cargo.toml
//!   .cargo/
//!     config.toml
//!   src/
//!     lib.rs
//!   xtask/
//!     Cargo.toml
//!     src/main.rs
//! ```
//!
//! To create it the `xtask`, you can use `cargo new xtask` in the root of your project, you can
//! then create the `.cargo/config.toml` that should contains the following:
//! ```toml
//! [alias]
//! xtask = "run --package xtask --"
//! ```
//!
//! You should be able to invoke your xtask with `cargo xtask <task>`. For more information, check
//! the [xtask][xtask] repository.
//!
//! [xtask]: https://github.com/matklad/cargo-xtask
//! [workspace_wrapper]: https://doc.rust-lang.org/cargo/reference/config.html#buildrustc-workspace-wrapper

use std::process::Command;

/// Sentinel environment variable used to distinguish wrapper invocations from normal xtask
/// invocations.
const ENV_KEY: &str = "XTASK_RUSTC_WRAPPER";

/// Handle a potential rustc wrapper invocation, then return.
///
/// Call this at the very **first** statement in your xtask `main` function. When Cargo is invoking
/// the xtask binary as a `RUSTC_WORKSPACE_WRAPPER`, this function runs `rustc -Awarnings
/// <original-args>` and terminates the process. When the xtask is invoked normally by the
/// developer, this function is a no-op and returns immediately, so the rest of `main` executes as
/// usual.
///
/// # Panics
///
/// Panics if the process is running as a rustc wrapper but the rustc path argument is missing or
/// if the rustc subprocess cannot be spawned.
///
/// # Example
///
/// ```rust,no_run
/// fn main() {
///     xtask_no_warnings::init();
///
///     // Your xtask logic starts here.
/// }
/// ```
pub fn init() {
    if std::env::var_os(ENV_KEY).is_none() {
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

/// Configures the current process to act as a workspace wrapper.
///
/// This sets two environment variables on the current process:
///
/// - `RUSTC_WORKSPACE_WRAPPER` - points to the current xtask executable so that Cargo routes
///   workspace member compilation through it.
/// - `XTASK_RUSTC_WRAPPER` - a sentinel that `init` uses to detect wrapper invocations.
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
/// ```rust,no_run
/// fn build(no_warnings: bool) {
///     let mut cmd = std::process::Command::new("cargo");
///     cmd.args(["build", "--release"]);
///
///     if no_warnings {
///         xtask_no_warnings::setup();
///     }
///
///     cmd.status().expect("cargo failed");
/// }
/// ```
pub fn setup() {
    let wrapper =
        std::env::current_exe().expect("cannot determine the path to the current executable");

    // SAFETY: `set_var` is not safe if called concurrently from multiple threads on Unix.
    // See https://doc.rust-lang.org/std/env/fn.set_var.html
    unsafe {
        std::env::set_var("RUSTC_WORKSPACE_WRAPPER", wrapper);
        std::env::set_var(ENV_KEY, "1");
    }
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
/// ```rust,no_run
/// fn build_without_warning() {
///     xtask_no_warnings::cargo_command()
///         .args(["build", "--release"])
///         .status()
///         .expect("cargo command failed");
/// }
/// ```
pub fn cargo_command() -> Command {
    setup();
    let cargo = std::env::var_os("CARGO").unwrap_or("cargo".into());
    Command::new(cargo)
}
