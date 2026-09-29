# Xcode FFI Plan

Goal: replace the demo-only Rust bridge with a Heartwood-focused UniFFI surface that the Swift app can use for real repository and node flows.

## Phase 1: Define the public FFI surface

- [x] Audit the current Rust bridge exports in `xcode/rustylib/src/lib.rs`
- [x] List the Swift app needs from Heartwood:
  - [x] version/build metadata
  - [x] repo and node ID normalization
  - [x] repository lookup and summaries
  - [x] node status and identity data
  - [x] local state/config access
  - [x] read-only operations first, write actions later
- [x] Decide which operations should stay in Rust and which should be thin Swift wrappers
- [x] Define the error model for Swift-friendly failures

## Phase 2: Build a Rust bridge layer

- [x] Keep `xcode/rustylib` as the UniFFI entrypoint
- [x] Add a dedicated internal bridge module or helper crate for Heartwood-facing logic
- [x] Reuse existing Heartwood crates instead of reimplementing logic in the bridge
- [x] Prefer simple UniFFI types: strings, records, enums, optionals, lists, and explicit errors
- [x] Avoid exposing opaque Rust internals directly unless a handle type is clearly justified

## Phase 3: Implement read-only APIs first

- [x] Expose build metadata and runtime version info
- [x] Expose repository ID and node ID parsing/normalization helpers
- [x] Add repository summary/read APIs
- [x] Add node summary/status APIs
- [x] Add tests for each exported function

## Phase 4: Add stateful and interactive APIs

- [x] Identify the minimal set of safe write operations needed by the Swift app
- [x] Add explicit input validation for every mutating call
- [x] Map Rust errors into stable UniFFI errors
- [x] Add tests for success and failure paths

## Phase 5: Refresh Swift bindings and app usage

- [x] Regenerate UniFFI Swift bindings from the Rust interface
- [x] Add a small Swift façade over the generated bindings
- [x] Replace demo UI calls in `swiftyapp/ContentView.swift`
- [x] Keep the app compiling after each bridge addition

## Phase 6: Verification and CI

- [x] Run `cd xcode && make rust`
- [x] Run `cd xcode && make app`
- [x] Verify the generated bindings contract checks still pass
- [x] Add or update CI so the bridge build stays covered
- [x] Ensure generated artifacts are rebuilt, not edited manually

## Phase 7: Protocol gap — node runtime status

- [x] Add `heartwoodNodeStatus()` returning `HeartwoodNodeStatus { running, socket }`
- [x] Uses `radicle::Node::new(socket).is_running()` via the `Handle` trait
- [x] Add `heartwoodRoutingSummary()` returning `HeartwoodRoutingSummary { entries, seededRepos }`
- [x] Reads from `profile.routing()` using the `Store` trait
- [x] Add Swift cards for status and routing summary
- [x] Add tests for shape and error paths

## Phase 8: Protocol gap — COB read access

- [x] Add `heartwoodRepositoryIssueCounts(rid)` returning `HeartwoodIssueCounts { open, closed, total }`
- [x] Add `heartwoodRepositoryPatchCounts(rid)` returning `HeartwoodPatchCounts { open, draft, archived, merged, total }`
- [x] Uses `Issues::open(repo, ReadOnly)` and `Patches::open(repo, ReadOnly)` for read-only access
- [x] Add per-repo issue/patch count badges in Swift repository cards
- [x] Add tests for invalid RID handling

## Phase 9: Protocol gap — policy read access

- [x] Add `heartwoodSeedPolicies()` returning `[HeartwoodSeedPolicy { rid, policy, scope }]`
- [x] Add `heartwoodFollowPolicies()` returning `[HeartwoodFollowPolicy { nid, alias, policy }]`
- [x] Add `heartwoodIsSeeding(rid)` boolean
- [x] Add `heartwoodIsFollowing(nid)` boolean
- [x] Reads from `profile.policies_mut()` (read methods available on `Store<T>`)
- [x] Add Swift cards for seeding policies and followed nodes
- [x] Add tests for shape and invalid input handling

## Phase 10: Protocol gap — network sessions and per-repo seed count

- [x] Add `heartwoodNodeSessions()` returning `[HeartwoodSession { nid, link, addr, state }]`
- [x] Returns empty list when node is not running (graceful degradation)
- [x] Add `heartwoodRepositorySeedCount(rid)` returning `u64`
- [x] Uses `routing.count(&rid)` for efficient seed count lookup
- [x] Add Swift sessions card and per-repo seed count badge
- [x] Add tests for shape and invalid input handling

## Phase 11: Protocol gap — repository remotes and branches

- [x] Add `heartwoodRepositoryRemotes(rid)` returning `[HeartwoodRemote { nid, refs: [{name, oid}] }]`
- [x] Add `heartwoodRepositoryBranches(rid)` returning `[HeartwoodRef { name, oid }]`
- [x] Uses `RemoteRepository::remotes()` and `ReadRepository::references_glob()`
- [x] Add Swift remote count and branch list badges in repository cards
- [x] Add tests for invalid RID handling

## Phase 12: Protocol gap — notifications

- [x] Add `heartwoodNotificationCount()` returning `u64`
- [x] Add `heartwoodNotificationCountsByRepo()` returning `[HeartwoodNotificationCount { rid, count }]`
- [x] Reads from `profile.notifications_mut()` (read methods available on `Store<T>`)
- [x] Add Swift notification card with total count and per-repo breakdown
- [x] Add tests for shape

## Phase 13: Protocol gap — alias lookup

- [x] Add `heartwoodAliasForNode(nid)` returning `Option<String>`
- [x] Add `heartwoodNodesForAlias(alias)` returning `[String]`
- [x] Uses `profile.aliases()` which combines policy and database aliases
- [x] Add Swift alias lookup fields in the lookup card
- [x] Add tests for invalid input handling

## Phase 14: Protocol gap — repository commit log

- [x] Add `heartwoodRepositoryLog(rid, limit)` returning `[HeartwoodCommit { oid, message, author, timestamp }]`
- [x] Uses `ReadRepository::revwalk()` and `ReadRepository::commit()`
- [x] Add Swift recent commit list in repository cards
- [x] Add tests for shape and invalid RID handling

## Phase 15: Protocol gap — node inventory

- [x] Add `heartwoodNodeInventory()` returning `[String]` (list of RIDs)
- [x] Uses `routing.get_inventory(profile.id())` to get advertised repos
- [x] Add Swift inventory card showing advertised repositories
- [x] Add tests for shape

## Phase 16: Protocol gap — repository size

- [x] Add `heartwoodRepositorySize(rid)` returning `u64` (bytes)
- [x] Walks the repository directory to calculate total size
- [x] Add Swift size badge in repository cards (human-readable via ByteCountFormatter)
- [x] Add tests for shape and invalid RID handling

## Suggested implementation order

1. Metadata and normalization helpers
2. Repository lookup and summaries
3. Node identity and status helpers
4. Local state/config reads
5. Small, carefully scoped write operations
6. Swift UI integration
7. CI coverage and stabilization
8. Node runtime status and routing summary
9. COB read access (issues and patches)

## Notes

- Keep the bridge surface small and stable.
- Favor read-only APIs before write APIs.
- Treat generated UniFFI and Xcode outputs as derived artifacts.
- Use this file as the running checklist for the Xcode FFI work.
