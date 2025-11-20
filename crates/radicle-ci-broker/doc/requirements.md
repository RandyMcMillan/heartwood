Note: This is not finalized.

# Requirements for CI in Radicle

## Initial goal

Radicle itself, and Radicle users, need support for continuous
integration. Concretely, this means, at minimum, that when a change is
merged into the an integration branch, an automated process verifies
that the branch still fulfills any requirements put on it.

More concretely, for Radicle itself, and for projects using Radicle
that follow a similar development process, when a change is merged
into the `master` branch (or the `main` branch, depending on the
repository), all the code builds, and all the automated tests pass.

Simplified, the following command should pass in the Radicle
`heartwood` repository after every merge:

~~~
cargo test --locked --workspace
~~~

In addition to the `master` branch, the same checks are run on any
patch created with `rad patch` on the Radicle repositories.

Further, the any runs and their results (success vs failure) is
communicated via the Radicle network. If a contributor creates a
patch, they can query `rad` to see if the tests pass under CI. (If
there are logs from CI for a failing run, they have to be queried from
the node that runs CI, for now.)

This is an initial, minimal goal that's useful. We can build on this.

## Long-term goal

Later on, we will want to reach much more ambitious goals for
integration support in Radicle. Ideally, we should aim CI in Radicle
to surpass all other CI that has ever existed, but that's more of a
vision than a goal.

* We do more extensive checks than just building and running tests.
  For example, running clippy, checking formatting, checking for
  unmaintained or insecure dependencies (even indirect ones), etc.
* Allow projects using Radicle to freely decide what a "CI run" will
  do for their project.
* We run tests on a tentative merge, but don't publish it to the
  Radicle network unless CI is happy. This will hopefully prevent a
  broken change from even being merge, where the initial goal only
  detects it after being merged.
* We publish built artifacts, such as program binaries, installation
  packages, web site content, etc.

However, this all requires the foundation of the initial goal to work.

## Initial requirements

We want to support both external and native CI engines so that we
don't lock ourselves into supporting only one, even by mistake.

* Radicle can integrate with various external CI engines and
  instances.
  - We don't want to insist Radicle users use a specific engine,
    because whichever one we chose would be unwelcome to some Radicle
    users.
  - The first such engine we've chosen to support is Concourse.
  - Setting up and running the external engines is not part of
    Radicle.
* Radicle comes with at least a simplistic native CI engine.
  - This will not be sufficient for many other projects, but will be
    enough to at least run basic CI checks for Radicle itself, to
    start with.
* Native CI comes with Radicle and is extremely easy to set up.
  - it does not, however, need to be highly secure or performant
* A node can run CI for the repositories in its storage.
  - The node admin can opt in to run CI on their node.
  - The node admin can configure on which changes a CI run is
    triggered. A node might not have enough resources for CI to check
    arbitrary changes, but the owner might want to concentrate on
    specific ones, such as their own patches.
* A node should announce to the network what CI runs it does, on what
  commits, and what the result of each run is.
  - This allows people using other nodes to see that patches are OK in
    CI.
  - Just the metadata about each run, not build log or build
    artifacts, at least for now.
* We can set up `seed.radicle.xyz` that run CI on any patch on the
  `heartwood` repository and checks that the patch builds and its
  tests pass.
  - We can then set this up on other repositories on the seed node as
    well.
  - We are confident enough in our CI solution that this is reasonably
    safe and secure to do.
  - This can use an external CI or native CI, depending on the state
    of those at the time of setting up.
  - If we don't want to risk the central node with this, we can do it
    on another node instead.

## Requirements for a CI broker in Radicle

Based on what we've discussed and experimented with and done so far,
Radicle will have a "CI broker" component that acts as a bridge
between a Radicle node and a CI engine. The broker will support native
CI and a selection of external CI engines.

* Broker supports external CI engines.
  - first version supports at least Concourse
* Broker supports Radicle native CI.
* The internal interfaces for supporting a CI engine are the same for
  native CI and external CI.
  - this is necessary so that we don't embed unwanted assumptions in
    the interface
* Broker can be extended to support new engines.
* Broker listens to node events.
* Broker updates the Radicle network about CI runs.
* As much as is reasonably possible of the broker code is shared
  between different CI engines.

# Requirements for event filtering in the CI broker

The purpose of event filtering in the CI broker is to allow the node
admin, who runs the broker, useful control of what changes in the
repository trigger a CI run.

For all of the following use cases, "I" is the node admin.

* I want to only run CI on a specific repository so that I spend my CI
  resources on things that are important to me.
  - filter on repository id

* I want to only run CI on changes to a specific branch so that I can
  build and deliver artifacts from the branch. For example, I might
  build and publish binaries for a program, or build and update a web
  site.
  - filter on branch name

* I want to run CI on any proposed patches that are created or
  modified for a project so that I can provide feedback on whether the
  changes in the patch pass the project's test suite.
  - filter on any branches that refer to Radicle patches

* I want to run CI only on a specific patch, when it's updated, so
  that I can check that that patch is OK.
  - filter on the branch for a specific Radicle patch

* I want to filter on a complex condition based on a combination of
  several simpler conditions.
  - allow boolean expressions on filter conditions with AND, OR, NOT
