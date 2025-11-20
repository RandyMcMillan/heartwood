# Release notes for `radicle-ci-broker`

This file summarizes the user-visible changes to `radicle-ci-broker`
between releases.


## Version 0.7.0, released 2024-10-14

This release breaks many existing instances. The event filter
configuration needs to be reviewed upon upgrade, and the event queue
in the database needs to be emptied before running the new version.

### Breaking changes

* Internally, the CI broker now converts change events from the
  Radicle node into "CI events". In previous versions of the CI broker
  it used "broker events". CI events are different from broker events,
  and aim to be more suitable for CI purposes. This also affects the
  event filter in the CI broker configuration. See the new [user
  guide](https://app.radicle.xyz/nodes/radicle.liw.fi/rad:zwTxygwuz5LDGBq255RA2CbNGrz8/tree/doc/userguide.md)
  for details on the CI events.
  
  Note that in some simple cases, the old broker events match a new CI
  event exactly, for filtering. As an example, a filter on a specific
  branch is the same.
  
  The CI event changes only affect the configuration file and the
  database. The CI adapters and the messages communicating with them
  are not affected. Existing adapters work as before.

### Bug fixes

* The CI broker no longer logs the sensitive environment variables in
  clear text. Fix by Michalis.

* The CI broker no longer appends a "shut down" event to the event
  queue when it loses connection to the local Radicle node. This was a
  remnant of an old development phase, used to tell other threads in
  the CI broker to terminate. There is now a better way to do that and
  the spurious event is not needed. In fact, the spurious event would
  be processed by the CI broker after it got re-started, meaning the
  new process would immediately terminate. Ars long, vita brevis.

* The log message of the CI broker process ending with an
  unrecoverable error is not logged at the error level instead of the
  info level.

### Other changes

* A start of a CI broker user guide (aimed at node operators) has been
  added to the source repository. It will be published by the CI
  broker's CI on <https://pages.radicle.liw.fi/ci-broker/>.

* The CI broker now enables logging in trace and debug levels in
  release builds.

* CI broker logging level can be set with new `--log-level` command
  line option. This functionality was dropped when structured logging
  was added. Note that the old way of controlling logging level with
  an environment variable is still absent. Please open an issue if you
  want it back.

* The CI broker now logs a lot more detail. Possibly too much detail.

* The `cibtool event add --base` options value is not rev-parsed. This
  means a value like `HEAD` or `HEAD^` will work. Previously it had to
  be a commit SHA.

## Version 0.6.3, released 2024-09-16

A very small release that I make mostly to keep up a weekly release
cadence.

* Fix bug where CI broker would crash (panic) when generating HTML
  report pages. The cause was that it was assuming that each CI run
  had a run ID assigned by the CI adapter, and that they were unique.
  This was a faulty assumption. Luckily, all this code making the
  assumption had been made superfluous by earlier changes, so it could
  just be removed. This resulted in further simplification of the
  report generation code, but those should not be visible to users.

* Add the `cibtool event record` subcommand, to listen on node events
  and optionally write them to a file. As this uses the same code as
  `cib` to subscribe to node events, it's a way for node operators to
  verify that the CI broker can actually receive events.

* Add the `cibtool event broker` subcommand that reads a previously
  recorded (or constructed) file with node events and produces the
  corresponding broker events from them. This can be useful to create
  test files for verifying that event filters work as intended, for
  node operators.

* Add the `cibtool event filter` subcommand that reads a set of event
  filter expressions from one file, and broker events from another
  file, and writes out the events allowed by the filter. This is meant
  to be useful for node operators to verify that their event filters
  work as meant.


## Version 0.6.2, released 2024-09-10

A very small release that I make mostly to keep up a weekly release
cadence.

* `README.md` now documents dependencies that aren't Rust crates
  needed to run the test suite.

* A small bug fix to the acceptance test suite. The dummy adapter used
  in the suite did not read its stdin, so the adapter would sometimes
  finish before the CI adapter wrote the trigger message to the
  adapter's stdin, which failed, causing the whole test to fail.


## Version 0.6.1, released 2024-09-02

* This updates the dependencies on other Radicle crates, to make it
  easier to `cargo install` the CI broker.

## Version 0.6.0, released 2024-09-02

### Breaking changes
  
* The CI broker test suite now verifies that it can be upgraded
  without making changes to the configuration file or the database.
  This should lessen how often accidental, unintended breaking
  changes, so this counts as a user-visible lack of change. Note that
  this testing doesn't prevent breaking changes from being made, but
  makes it more likely that they'll be documented as such.

### New or changed features

* The CI broker now logs, to its own log, the standard error output of
  the CI adapter it runs, as well as their exist code. This should
  make it easier to debug problems in adapters, especially for the
  developers of the adapters. The log messages look like:
  
~~~json
{"msg":"adapter exit code","level":"DEBG","ts":"2024-08-27T17:38:26.762222386Z","exit_code":1}
{"msg":"adapter stderr","level":"DEBG","ts":"2024-08-27T17:38:26.762289242Z","stderr":"woe be me\n"}
~~~

* The `cibtool event list` command now has a `--jdon` option to output
  the event queue using the JSON format, for easier post-processing by
  other software.

* The CI broker logs its git commit when it starts.

* The CI broker [`README.md`](README.md) lists the known CI adapter,
  and links to documentation about using them.


## Version 0.5.0, released 2024-08-26

### Breaking changes

* All CI broker (`cib`) logging is now to the standard error output as
  JSON Lines: each log message is on its own line as one JSON object.
  This is a breaking change as it changes what is logged and how, and
  this may matter to those consuming the log output.

  The `RADICLE_CI_LOG` environment variable no longer affects CI
  broker logging.

* The `cibtool run add` option `--alias` has been removed. It was
  mis-named (repositories have names, not aliases), and also
  unnecessary, as the program can look up the repository name itself.
  Related to this, the JSON output calls it `repo_name` instead of
  `repo_alias`. However, when reading run information stored in the
  database, the old name is still accepted.

### New or changed features

* The `cibtool run add` option `--repo` can now take a repository
  name, instead of its id, if there is exactly one repository with
  that name in the local node.

* The HTML report front page now lists the current queue of
  unprocessed events, except for events for private repositories. The
  JSON status page includes the length of the queue.


## Version 0.4.0, released 2024-08-19

This is the fourth release. In breaking with tradition, it does not
have significant breaking changes.

### Breaking changes

* The `cibtool run list` command now lists only the identifiers of CI
  runs, one per line, by default. An option `--json` was added to get
  all the information in a format that's easy to parse with a program.
  This is a breaking change if you parse the output of the command
  with another command.

* The `cibtool run add` command no longer has a `--finished` option.
  It is now enough to use `--successful` or `--failure`. The dropped
  option was always used with one of those two, and those two were
  always used with the dropped option, so the dropped one was
  pointless and provided no extra information. It was just extra work
  for the user.

### Bug fixes

* The CI broker handles errors without panicking now. Instead of a
  stack trace, an actual error message is shown. This may be more
  friendly to most people.

* The CI broker now depends on Radicle 0.12.0, to say more current
  with changes in Radicle.

* The `cibtool run add` command sets the state correctly if the run
  has finished. Previously it was set to "triggered".

* Several instances of "CI" being spelled as "Ci" have been fixed, in
  log messages and help texts.


### New or changed features

* The HTML report pages produced by the CI broker are now a little
  more readable.

* The various parts of the running CI broker now notify each other
  that there have been changes so that the other parts can do what
  they need to do at once, instead of checking on a fixed schedule.
  This means, for example, that the HTML report pages for a run are
  updated at once, instead of once a minute.

* The `cib` and `cibtool` commands now both have an option `--version`
  to show the version number of the program.

* The `cibtool trigger` command makes it easier to trigger a CI run.
  This duplicates the functionality of `cibtool event add`, but is a
  little simpler to remember, and a little safer to use. The trigger
  command makes sure the base commit is always set in the event it
  creates. (The command to add an event does not, in cases users
  really want that behavior.)

* Each CI run now has two identifiers: one invented by the CI broker
  itself, guaranteed to be unique, and one chosen by the adapter,
  meant to reflect the identifier used by an external CI system, and
  guaranteed to be unique. Everywhere the user refers to a CI run,
  either ID is accepted, as long as the adapter one is unambiguous.
  When a run ID is shown, both kinds are shown.

* During a CI run, when anything happens, the information about the
  run  is updated in the database at once, instead of at the end of
  the run. This means the change is not lost in case the CI broker is
  terminated before the run finishes.

* New commands `cibtool run show` and `cibtool run update` to show
  information about a run, and update it. The updating is probably
  mostly only relevant to those writing test suites for the CI broker.

* Started documenting the acceptance criteria for logging in the CI
  broker, and how to verify that the criteria are met. This will
  probably continue over the next few releases. As part of this, the
  stakeholders are documented. In this release the list of
  stakeholders is quite rudimentary and the CI broker developers would
  like to hear from you if you have any wants or needs about the CI
  broker, whether about logging or otherwise.

  Additionally, this takes the first small steps towards having
  structured logging.


## Version 0.3.0, released 2024-07-18

This is the third release. It breaks all existing installations.

### Bug fixes

* The CI broker now parses Git ref names for node identifiers more
  correctly.

* When the node sends an event about a branch having changed, the CI
  broker now allows the branch name to have slashes.

### New or changed features

* The `ci-broker` program has been replaced with the `cib` program,
  which is better in a few ways. Most importantly, it persists the
  node events that need to be processed, in a queue, so that they are
  not lost when the CI broker is re-started. The persistent storage is
  in the form of an SQLite database file: `ci-broker` used it already,
  but `cib` puts more information in it. There is also a list of all
  CI runs `cib` has performed.

* The `cibtool` program has been added to allow the node operator to
  manage `cib`, via the SQLite database file. `cibtool` lets the
  operator to view and manipulate the node event queue, and the list
  of CI runs, and to generate HTML report pages from information in
  the database. (`cib` generates the same report pages when running:
  `cibtool` functionality is there just in case.)

* CI broker logging continues to be inconsistent and helpful mainly to
  those developing the CI broker itself. Some logging has been
  removed, to avoid so called logspam.

* The code has been refactored a lot, for simplicity. This should be
  invisible to others, especially the adapter implementations, but
  just in case, be aware something externally visible may have changed
  inadvertently.

* The `BrokerEvent::is_allowed` method is no longer exposed in the CI
  broker public API. If you need it, let us know.

* The CI broker now logs the executable filename of the adapter it
  runs.

* The acceptance criteria for the CI broker are now documented in more
  detail in the subplot (see `ci-broker.md` and the
  [Subplot](https://subplot.tech/) software). The subplot document is
  available as
  [HTML](https://pages.radicle.liw.fi/ci-broker/ci-broker.html).

* Various undocumented helper programs have been dropped from the CI
  broker. Their functionality, if needed, can be added to `cibtool` in
  the future.


## Version 0.2.0, released 2024-06-06

This is the second release.

### Bug fixes

* The CI broker now picks the newest commit in a patch or branch,
  rather than the oldest. This means changes to a patch or branch get
  processed by CI, rather than only the first version. (Lars
  Wirzenius)

* Error messages are a little clearer now. An unclear error message is
  a bug. (Lars Wirzenius)

* Problems with the adapter sub-process failing are handled better
  now. (Lars Wirzenius)

* If the CI broker loses its connection to the Radicle node, the
  broker now terminates. Previously, it would keep trying to receive a
  message from the broken connection, which it would never receive,
  since the connection was broken in a way that it would not be mended
  even if the node is restarted. Computers are very patient, and never
  give up hope, so the CI broker was willing to wait until the heat
  death of the universe, or a power outage, whichever came first.

  Now, something like `systemd` can be used to restart the CI broker
  if this happens, and the system will recover. (Lars Wirzenius)


### New or changed features

* The CI broker depend on the version of the `radicle` crate that
  corresponds to the Radicle (heartwood) version 1.0.0 release
  candidate 10. This makes it possible to use the CI broker with the
  current version of Radicle. (Lars Wirzenius)

* The adapter protocol now allows an adapter to give the CI broker a
  URL related to the CI run. This can be used to link to a build log,
  for example. (Lars Wirzenius)

* The node event filter now has an `AnyPushRef` condition. (Michalis
  Zampetakis)

* The CI broker architecture documentation is now published to
  <https://pages.radicle.liw.fi/ci-broker/architecture.html> by CI
  whenever the CI broker is changed. (Lars Wirzenius)

* The logging for event handling a lot more verbose. This is useful
  for debugging problems, but is otherwise so noisy logs become nearly
  useless. (Lars Wirzenius)

* The documentation for the broker/adapter protocol should be up to
  date. (Michalis Zampetakis)

* The CI broker configuration now has the `sensitive_envs` field to
  allow specifying environment variables that should not be logged.
  (Lars Wirzenius)

* The CI broker can now be shut down cleanly. This is mostly useful
  for testing, though, at least for now, but opens up the possibility
  of shutting down only after any in-progress CI runs having finished
  first. (Lars Wirzenius)


## Version 0.1.0, released 2024-04-03

This was the first release. No notes.
