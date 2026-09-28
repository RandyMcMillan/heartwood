# Copilot instructions for Heartwood

Heartwood is a Rust workspace for the Radicle stack. The main pieces are:

- `crates/radicle`: shared library code used across the stack.
- `crates/radicle-cli`: the `rad` CLI entrypoint and subcommands.
- `crates/radicle-node`: the `radicle-node` daemon and networking/runtime logic.
- `crates/radicle-remote-helper`: the `git-remote-rad` Git remote helper.
- `crates/radicle-cli-test`: markdown-driven CLI test harness.
- `simulation/`, `systemd/`, and `windows/`: simulation, service, and packaging support.
- `vendor/`: vendored third-party crates; treat as dependency code, not product code.

## Build, test, and lint

- `just` — list available repo tasks.
- `just format-rust` — check Rust formatting.
- `just check-rust` — `cargo check --workspace --all-targets --all-features`.
- `just lint-rust` — `cargo clippy --workspace --all-targets --all-features -- --deny warnings`.
- `just test-rust` — `cargo nextest run --workspace --all-features --no-fail-fast`.
- `cargo test --workspace` — fallback full test run when nextest is not available.
- `just check-docs` — build docs with `RUSTDOCFLAGS="--deny warnings"`.
- `just pre-commit` — run the full pre-commit suite used by hooks.
- `just install-hooks` — install the local git hooks when not using Nix.
- `just format-nix` — format Nix files when touching `flake.nix` or related files.

Single-test examples:

- `cargo test -p <crate> <test_name> -- --exact`
- `cargo nextest run -p <crate> <test_name>` if you prefer nextest filtering

Useful run commands:

- `cargo run -p radicle-cli --bin rad -- <args>`
- `cargo run -p radicle-node -- <args>`
- `cargo run -p radicle-remote-helper --bin git-remote-rad -- <args>`

## High-level architecture

Heartwood is a layered workspace rather than one monolith. `radicle` holds shared domain and storage logic, while supporting crates such as `radicle-core`, `radicle-crypto`, `radicle-cob`, `radicle-dag`, `radicle-protocol`, `radicle-term`, and `radicle-surf` split reusable concerns out of the binaries. The CLI (`rad`) is a thin command dispatcher over those libraries, the node daemon owns peer-to-peer/network behavior, and the Git remote helper bridges Git operations to Radicle storage and node notification.

`radicle-cli-test` is the test harness for CLI documentation examples, so CLI changes often need both a markdown example and a corresponding Rust test. The repo also contains system integration surfaces: `systemd/` for service units, `simulation/` for local network simulation, and `windows/` for Windows packaging.

## Key conventions

- Follow `CONTRIBUTING.md` and `HACKING.md` for code style and workflow.
- Keep imports grouped from `std`, then external crates, then local crates, then `super`, with module declarations before imports.
- Use `unwrap`/`expect` sparingly; tests are the main acceptable exception.
- Logging should include an explicit `target` and enough context to stand alone.
- Public types and functions should be documented.
- Avoid adding dependencies unless there is a strong reason.
- Do not hardcode `radicle.dev` in code unless there is a clear justification.
- Keep changes to `vendor/` to dependency updates only.
- Use repository isolation helpers when running local experiments: `RAD_HOME`, `RAD_KEYGEN_SEED`, and `RAD_PASSPHRASE` are the main environment variables documented for development.
- For CLI work, add or update the markdown example in `crates/radicle-cli/examples` and the matching unit test in `crates/radicle-cli/tests/commands.rs`.
