# radicle-ci-broker

Add integration to CI engines/systems/services to
[Radicle](https://radicle.xyz/), a distributed git hosting and
collaboration system.

This is not quite production ready code yet, but it will eventually become a
thing that listens for changes in a Radicle node, and triggers CI on
the relevant ones.


## Architecture

See the `doc` directory for architecture documentation. Quick summary:
the CI broker gets events from the Radicle node, filters them based on
its own config, and for any event that gets past the filter, runs the
configured adapter executable. The broker and adapter use a simple
JSON based message protocol over stdin/stdout. Each CI system has its
own adapter.

To build the documentation, run `make` in the `doc` directory. You'll
need some tools installed: Pandoc, graphviz (dot), PlantUML,
pikchr-cli. The others are widely packaged, pikchr-cli is a Rust
crate, so you can install it with `cargo install pikchr-cli`

Build and publish the documentation like this:

~~~
RADICLE_CI_BROKER_WEBROOT=/tmp/ci make -C doc publish
~~~


## Binaries

The crate contains several binaries:

* `cib` --- the actual CI broker
  - this is the only one you need to care about, the rest are for
    debugging
* `cibtool`  --- management tool for node operators, for managing the
  CI broker
* `synthetic-events`  --- test tool for CI broker developers


## Packaging

There is simple, simplistic, rudimentary, personal-use-only packaging
for Debian in the `debian` directory, used by Lars to build packages
for his own use.


## Running from the source tree

To run `cib` or `cibtool` from the source tree:

~~~sh
cargo run --bin cib -- --config config.yaml process-events
cargo run --bin cibtool -- --db ci-broker.db event list
~~~

Note the `--` argument. It tells `cargo run` that all the arguments
that follow are to be passed to the program being run, and not used by
`cargo` itself.


## Running tests

To run the test suite for the CI broker:

~~~sh
cargo test
~~~

(The usual way, for a Rust program.)

In addition to the usual Unix command line tools, you need the
following programs installed for the test suite:

* `jq`
* `git`
* `sqlite3`
* `rad` (from Radicle)

## Configuration

The configuration file is named on the command line. It is a YAML
file, for example:

~~~yaml
default_adapter: native
db: ci-broker.sqlite
adapters:
  native:
    command: radicle-native-ci
    env:
      RADICLE_NATIVE_CI: /home/liw/radicle/radicle-native-ci/x/config.yaml
    sensitive_env:
      some_secret: some_secret_value_that_is_not_logged
filters:
  - !And
    - !Repository "rad:z2e6URdt1we1iG1BCVqtx8QVgsX4a"
    - !Or
      - !Branch "main"
      - !AnyPatchRef
~~~

Or if you only want to filter for patch COB updates (e.g. comments),
to the specified repository:

~~~yaml
default_adapter: native
db: ci-broker.sqlite
adapters:
  native:
    command: radicle-native-ci
    env:
      RADICLE_NATIVE_CI: /home/liw/radicle/radicle-native-ci/x/config.yaml
    sensitive_env:
      some_secret: some_secret_value_that_is_not_logged
filters:
  - !And
    - !Repository "rad:z2e6URdt1we1iG1BCVqtx8QVgsX4a"
    - !And
      - !AnyPatch
      - !Not 
        - !AnyPatchRef 
~~~

This runs the native CI engine as an adapter, on any repository events
that pass the filter. The filter allows any changes to the `main`
branch or any Radicle patch, on the specified repository.


## Adapters

You need to use an "adapter" together with the CI broker to actually
run CI on projects. At least the following adapters for external CI
systems are known:

* [Radicle native
  CI](https://app.radicle.xyz/nodes/seed.radicle.xyz/rad:z3qg5TKmN83afz2fj9z3fQjU8vaYE)
  - Run CI on the node directly, using `.radicle/native.yaml`
* [Concourse integration](https://app.radicle.xyz/nodes/seed.radicle.garden/rad:z2woyw9Get9Q21VJzdbVz33b47xDb)
  - Integrate with the Concourse CI system
* [GitHub actions](https://explorer.radicle.gr/nodes/seed.radicle.gr/rad:z4Uh671FzoooaHjLvmtW9BtGMF9qm/tree/Node-Operator/GitHub-Actions-Integration.md)
  - Integrate with GitHub actions
* [Outgoing web hooks](https://explorer.radicle.gr/nodes/seed.radicle.gr/rad:z4Uh671FzoooaHjLvmtW9BtGMF9qm/tree/Node-Operator/Webhooks-Integration.md)
  - Call a remote web hook when an event occurs in a repository hosted
    on the Radicle network
* [Kraken](https://kraken.ci/blog/integration-with-radicle/)
  - Integrate with the Kraken CI system
* [Zulip Integration](https://explorer.radicle.gr/nodes/seed.radicle.gr/rad:z4Uh671FzoooaHjLvmtW9BtGMF9qm/tree/Node-Operator/Zulip-Integration.md)
  - Integrate with Zulip chat for getting informed about new events on repos
  
See also the
[radicle-ci-integrations-docs](https://explorer.radicle.gr/nodes/seed.radicle.gr/rad:z4Uh671FzoooaHjLvmtW9BtGMF9qm)
repository with guides for project maintainers and node operators
about using Radicle CI. They too list CI adapters.


## License

Radicle CI broker is distributed under the terms of both the MIT
license and the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.
