# Xcode FFI Plan

Goal: replace the demo-only Rust bridge with a Heartwood-focused UniFFI surface that the Swift app can use for real repository and node flows.

## Phase 1: Define the public FFI surface

- [ ] Audit the current Rust bridge exports in `xcode/rustylib/src/lib.rs`
- [ ] List the Swift app needs from Heartwood:
  - [ ] version/build metadata
  - [ ] repo and node ID normalization
  - [ ] repository lookup and summaries
  - [ ] node status and identity data
  - [ ] local state/config access
  - [ ] read-only operations first, write actions later
- [ ] Decide which operations should stay in Rust and which should be thin Swift wrappers
- [ ] Define the error model for Swift-friendly failures

## Phase 2: Build a Rust bridge layer

- [ ] Keep `xcode/rustylib` as the UniFFI entrypoint
- [ ] Add a dedicated internal bridge module or helper crate for Heartwood-facing logic
- [ ] Reuse existing Heartwood crates instead of reimplementing logic in the bridge
- [ ] Prefer simple UniFFI types: strings, records, enums, optionals, lists, and explicit errors
- [ ] Avoid exposing opaque Rust internals directly unless a handle type is clearly justified

## Phase 3: Implement read-only APIs first

- [ ] Expose build metadata and runtime version info
- [ ] Expose repository ID and node ID parsing/normalization helpers
- [ ] Add repository summary/read APIs
- [ ] Add node summary/status APIs
- [ ] Add tests for each exported function

## Phase 4: Add stateful and interactive APIs

- [ ] Identify the minimal set of safe write operations needed by the Swift app
- [ ] Add explicit input validation for every mutating call
- [ ] Map Rust errors into stable UniFFI errors
- [ ] Add tests for success and failure paths

## Phase 5: Refresh Swift bindings and app usage

- [ ] Regenerate UniFFI Swift bindings from the Rust interface
- [ ] Add a small Swift façade over the generated bindings
- [ ] Replace demo UI calls in `swiftyapp/ContentView.swift`
- [ ] Keep the app compiling after each bridge addition

## Phase 6: Verification and CI

- [ ] Run `cd xcode && make rust`
- [ ] Run `cd xcode && make app`
- [ ] Verify the generated bindings contract checks still pass
- [ ] Add or update CI so the bridge build stays covered
- [ ] Ensure generated artifacts are rebuilt, not edited manually

## Suggested implementation order

1. Metadata and normalization helpers
2. Repository lookup and summaries
3. Node identity and status helpers
4. Local state/config reads
5. Small, carefully scoped write operations
6. Swift UI integration
7. CI coverage and stabilization

## Notes

- Keep the bridge surface small and stable.
- Favor read-only APIs before write APIs.
- Treat generated UniFFI and Xcode outputs as derived artifacts.
- Use this file as the running checklist for the Xcode FFI work.
