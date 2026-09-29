# Copilot instructions for Heartwood

Heartwood is the Radicle Rust workspace, plus supporting simulation, Windows, and Xcode/Swift integration under `simulation/`, `windows/`, and `xcode/`.

## Build, test, and lint

Use the repo task runner first:

- `just` or `just --list` to inspect available tasks
- `just check-rust` to run `cargo check --workspace --all-targets --all-features`
- `just lint-rust` to run `cargo clippy --workspace --all-targets --all-features -- --deny warnings`
- `just test-rust` to run `cargo nextest run --workspace --all-features --no-fail-fast`
- `just pre-commit` for the full pre-commit suite
- `just check-docs` for docs warnings
- `just format-rust` to verify formatting
- `just install-hooks` to install local git hooks

Useful Cargo examples:

- `cargo test --workspace` if `nextest` is unavailable
- `cargo test -p radicle-cli --test commands rad_clone -- --exact` for a single CLI integration test
- `cargo test -p radicle-node inventory_sync_basic -- --exact` for a single node test
- `cargo test -p radicle-node --lib e2e -- --nocapture` for the node e2e suite
- `cargo check -p radicle-node` or `cargo check -p radicle-cli` for a focused compile check

Xcode/Swift bridge commands:

- `cd xcode && make rust` to build the UniFFI Rust library and XCFramework
- `cd xcode && make resolve` to resolve Swift package dependencies
- `cd xcode && make app` to build the iOS app
- `cd xcode && make catalyst` to build the Mac Catalyst app
- `cd xcode && make clean` to remove Rust and Xcode build output
- `cd xcode/rustylib && cargo check` to verify the UniFFI bridge crate alone

## High-level architecture

The workspace is layered rather than monolithic. `crates/radicle` holds shared domain and storage logic; `crates/radicle-cli` is the `rad` command-line client; `crates/radicle-node` owns peer-to-peer networking and the node daemon; `crates/radicle-remote-helper` implements `git-remote-rad`; and `crates/radicle-cli-test` powers the markdown-driven CLI examples.

Integration surfaces live beside the Rust workspace: `simulation/` contains the Kubernetes/Timoni-based network simulator, `windows/` packages the Windows installer, and `xcode/` contains the UniFFI-based Swift bridge and sample app. The Xcode app is generated from `xcode/rustylib`, which builds the Rust library, emits the FFI bindings, and packages them into `RustyCore.xcframework` for the Swift package in `xcode/swiftyapp/Lib/swiftyrustlib`.

## Key conventions

- Follow `CONTRIBUTING.md` and `HACKING.md` for workflow and style.
- Keep imports ordered as `std`, external crates, local crates, then `super`.
- Public types and functions should be documented; logging should include a `target` and enough context to stand alone.
- Prefer existing task runner commands over ad hoc shell snippets.
- Use `RAD_HOME`, `RAD_KEYGEN_SEED`, `RAD_PASSPHRASE`, and `SOURCE_DATE_EPOCH` when reproducing or isolating runs.
- CLI changes usually need both the markdown example in `crates/radicle-cli/examples` and the matching Rust test in `crates/radicle-cli/tests/commands.rs`.
- Node logic tests live in `crates/radicle-node/src/tests.rs`; network/e2e coverage lives in `crates/radicle-node/src/tests/e2e.rs`.
- Treat `vendor/` as dependency code; change it only for dependency updates or vendor integrity fixes.
- Do not hand-edit generated UniFFI/Xcode outputs; regenerate them via `xcode/build.sh`.
- Release/version handling assumes `releases/*` tags, with fallback logic in the Rust build scripts and `windows/version.ps1` for non-tagged builds.
