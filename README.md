# Xtask-no-warnings

Suppress warnings in [xtask][xtask] builds without invalidating the dependency cache.

## Purpose

The standard way to suppress compiler warnings during development is to set `RUSTFLAGS=-Awarnings`.
It works but it has a painful side effect.

`RUSTFLAGS` is part of the compiler fingerprint for **every** crate in the build graph. Toggling it
at any point forces Cargo to recompile the entire project from scratch. On machines with limited
resources (e.g. low-specs or handled/laptop computers) this can mean minutes of wasted build time
every single time you flip the flag.

This crate solves the problem by using [`RUSTC_WORKSPACE_WRAPPER`][workspace_wrapper] instead of
`RUSTFLAGS`.

`RUSTC_WORKSPACE_WRAPPER` routes `rustc` invocations through a wrapper binary but **only for workspace
members**. Dependencies are compiled by `rustc` directly so their fingerprints never change and
their cached artifacts remain valid regardless of whether the wrapper is active.

The wrapper here is the xtask binary itself. At startup, `init` checks for a sentinel
environment variable. When Cargo invokes the xtask as a rustc wrapper, `init` forwards all arguments
to the real `rustc` with `-Awarnings` prepended and then exits. When the developer invokes the xtask
normally, `init` is a no-op and the rest of `main` runs a usual.

Because `RUSTC_WORKSPACE_WRAPPER` produces artifacts under a **separate fingerprint** from a plain
`rustc` run, the two modes (warnings on or off) maintain independent caches for workspace members.
The very first toggle in each direction recompiles those crates but every subsequent toggle hits the
cache immediately.

## Usage

### 1. Add the dependency to your xtask

```toml
# xtask/Cargo.toml
[dependencies]
xtask-no-warnings = "0.1"
```

### 2. Call `init` at the top of main

```rust
// xtask/src/main.rs
fn main() {
    xtask_no_warnings::init();

    // Your xtask logic here.
}
```

`init` must be the very first statement so that when Cargo invokes the xtask as a rustc wrapper it
exits immediately, before any of your setup code runs.

### 3. Use the wrapper when spawning Cargo

#### Option A - `cargo_command` function

Returns a `Command` for Cargo with the wrapper environment variable already set. Append your
subcommand and flags before running it.

```rust
fn build(no_warnings: bool) {
    let mut cmd = if no_warnings {
        xtask_no_warnings::cargo_command()
    } else {
        std::process::Command::new(std::env::var_os("CARGO").unwrap_or("cargo".into()))
    };

    cmd.args(["build", "--release"])
        .status()
        .expect("cargo failed");
}
```

#### Option B - `setup` function

Configures an existing `Command` in place. Useful when you are building the `Command` yourself and
only want to add the wrapper conditionally.

```rust
fn build(no_warnings: bool) {
    let mut cmd = std::process::Command::new("cargo");
    cmd.args(["build", "--release"]);

    if no_warnings {
        xtask_no_warnings::setup(&mut cmd);
    }

    cmd.status().expect("cargo failed");
}
```

[xtask]: https://github.com/matklad/cargo-xtask
[workspace_wrapper]: https://doc.rust-lang.org/cargo/reference/config.html#buildrustc-workspace-wrapper
