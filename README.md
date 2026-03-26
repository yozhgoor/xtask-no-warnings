[![actions status][actions-badge]][actions-url]
[![crate version][crates-version-badge]][crates-url]
[![documentation][docs-badge]][docs-url]
![licenses][licenses-badge]

[actions-badge]: https://github.com/yozhgoor/xtask-no-warnings/actions/workflows/rust.yml/badge.svg
[actions-url]: https://github.com/yozhgoor/xtask-no-warnings/actions
[crates-version-badge]: https://img.shields.io/crates/v/xtask-no-warnings
[crates-url]: https://crates.io/crates/xtask-no-warnings
[docs-badge]: https://docs.rs/xtask-no-warnings/badge.svg
[docs-url]: https://docs.rs/xtask-no-warnings/
[licenses-badge]: https://img.shields.io/crates/l/xtask-no-warnings

# xtask-no-warnings

Silence warnings in [xtask][xtask] builds without invalidating the dependency cache.

## Purpose

This is a micro crate with zero dependencies for use with xtask during development.

The standard way to silence compiler warnings during development is to set
`RUSTFLAGS=-Awarnings`. It works, but it has a painful side effect: `RUSTFLAGS` is part of the
compiler fingerprint for **every** crate in the build graph. Toggling it forces Cargo to
recompile the entire project from scratch, including all dependencies. On machines with limited
resources (e.g. low-specs laptops, handheld devices, ...) this means minutes of wasted build
time every single time you flip the flag.

This crate solves the problem by using [`RUSTC_WORKSPACE_WRAPPER`][workspace_wrapper] instead.
`RUSTC_WORKSPACE_WRAPPER` routes `rustc` invocations through a wrapper binary but **only for
workspace members**. Dependencies are compiled by `rustc` directly and their cached artifacts
remain valid regardless of whether the wrapper is active.

The wrapper here is the xtask binary itself. At startup, [`init`] checks for a sentinel
environment variable. When Cargo invokes the xtask as a rustc wrapper, `init` forwards all
arguments to the real `rustc` with `-Awarnings` prepended and then exits. When the developer
invokes the xtask normally, `init` is a no-op and the rest of the `main` function runs as
usual.

Because `RUSTC_WORKSPACE_WRAPPER` produces artifacts under a **separate fingerprint** from a
plain `rustc` run, the two modes (warning on or off) maintain independent caches for workspace
members. The very first toggle in each direction recompiles those crates, every subsequent
toggle hits the cache immediately.

## Usage

### 1. Add the dependency to your xtask

`xtask/Cargo.toml`
```toml
[dependencies]
xtask-no-warnings = "0.1"
```

### 2. Call init at the top of main

`xtask/src/main.rs`
```rust
fn main() {
    xtask_no_warnings::init();

    // Your xtask logic here.
}
```

`init` must be the very first statement so that when Cargo invokes the xtask as a rustc
wrapper, it exits immediately before any of your setup code runs.

### 3. Spawn Cargo with or without warnings

#### Option A - `cargo_command`

This function returns a `Command` for Cargo with the wrapper environment variable already set.
Append your subcommand and flags before running it.

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

#### Option B - `setup`

This function configures the current process to act as a workspace wrapper. Useful when you are
building the `Command` yourself and only want to add the wrapper conditionally.


```rust
fn build(no_warnings: bool) {
    let mut cmd = std::process::Command::new("cargo");
    cmd.args(["build", "--release"]);

    if no_warnings {
        unsafe { xtask_no_warnings::setup(); }
    }

    cmd.status().expect("cargo failed");
}
```

### Basic xtask setup

A typical project using a xtask workspace member looks like this:
```toml
my-project/
  Cargo.toml
  .cargo/
    config.toml
  src/
    lib.rs
  xtask/
    Cargo.toml
    src/main.rs
```

To create it the `xtask`, you can use `cargo new xtask` in the root of your project, you can
then create the `.cargo/config.toml` that should contains the following:
```toml
[alias]
xtask = "run --package xtask --"
```

You should be able to invoke your xtask with `cargo xtask <task>`. For more information, check
the [xtask][xtask] repository.

[xtask]: https://github.com/matklad/cargo-xtask
[workspace_wrapper]: https://doc.rust-lang.org/cargo/reference/config.html#buildrustc-workspace-wrapper
