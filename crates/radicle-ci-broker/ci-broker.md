# Introduction

This document describes the acceptance criteria for the Radicle CI
broker, as well as how to verify that they are met. Acceptance
criteria here means a requirement that must be met for the software to
be acceptable to its stakeholders.

This file is used by [Subplot](https://subplot.tech/) to generate and
run test code as part of running `cargo test`.

# Stakeholders

For the purposes of this document, a stakeholder is someone whose
opinion matters for setting acceptance criteria. The CI broker has the
following stakeholders, grouped so that specific people only need to
be named in one place:

* `cib-devs` &ndash; the people who develop the CI broker itself
  - Lars Wirzenius
* `adapter-devs` &ndash; the people who develop adapters
  - Lars Wirzenius
  - Michalis
  - Yorgos Saslis
* `node-ops` &ndash; the people operating a Radicle node, when they
  also run Radicle CI on it
  - Lars Wirzenius
  - Yorgos Saslis
* `devs` &ndash; the people for whose repositories Radicle CI runs;
  this means the people who contribute to any repository hosted on
  Radicle, when any node runs CI for that repository, as opposed to
  the people who develop the Radicle CI software
  - Lars Wirzenius
  - Michalis

Some stakeholders are named explicitly so that it will be easier to
ask them more information that is captured in this document. Note that
the list will evolve over time. Please suggest missing stakeholders to
the developers and maintainers of the CI broker.

# Data files shared between scenarios

## Broker configuration

~~~{#broker.yaml .file .yaml}
db: ci-broker.db
report_dir: reports
default_adapter: mcadapterface
adapters:
  mcadapterface:
    command: ./adapter.sh
    env:
      RADICLE_NATIVE_CI: native-ci.yaml
    sensitive_env: {}
filters:
  - !Branch "main"
~~~

~~~{#broker-allow-nothing.yaml .file .yaml}
db: ci-broker.db
report_dir: reports
default_adapter: mcadapterface
adapters:
  mcadapterface:
    command: ./adapter.sh
    env:
      RADICLE_NATIVE_CI: native-ci.yaml
    sensitive_env: {}
filters:
  - !Branch "this-branch-does-not-exist"
~~~

## A dummy adapter

This adapter does nothing, just reports a run ID and a successful run.

Note that this adapter always outputs a message to its standard error
output, even though it doesn't fail. This is useful for verifying that
the CI broker logs adapter error output, and doesn't harm other uses
of the adapter.

~~~{#dummy.sh .file .sh}
#!/bin/bash
set -euo pipefail
cat > /dev/null
echo '{"response":"triggered","run_id":{"id":"xyzzy"}}'
echo '{"response":"finished","result":"success"}'
echo "This is an adapter error: Mordor" 1>&2
~~~

## A `refsFetched` node event

This is a node event from the node to signal that some git refs have
changed in repository.

~~~{#refsfetched.json .file .json}
{
  "type": "refsFetched",
  "remote": "z6MkgEMYod7Hxfy9qCvDv5hYHkZ4ciWmLFgfvm3Wn1b2w2FV",
  "rid": "rad:zwTxygwuz5LDGBq255RA2CbNGrz8",
  "updated": [
    {
      "updated": {
        "name": "refs/namespaces/DUMMYNID/refs/heads/main",
        "old": "0000000000000000000000000000000000000000",
        "new": "0000000000000000000000000000000000000000"
      }
    }
  ]
}
~~~

## Set rid in `refsFetched` event

This is a helper script that reads a `refsFetched` event and changes
its `rid` field to be the id of the given repository. It also sets the
`name` field for updated refs to include the repository ID. It writes
the message back to its file.

~~~{#set-rid .file .python}
#!/usr/bin/python3

import json, sys
from subprocess import run, PIPE

filename = sys.argv[1]
cwd = sys.argv[2]

p = run(["rad", "."], check=True, capture_output=True, cwd=cwd)
rid = p.stdout.decode().strip()

p = run(["rad", "self", "--nid"], check=True, capture_output=True, cwd=cwd)
nid = p.stdout.decode().strip()

p = run(["git", "rev-parse", "HEAD"], check=True, capture_output=True, cwd=cwd)
oid = p.stdout.decode().strip()

o = json.load(open(filename))

o["rid"] = rid

if "updated" in o:
    x = o["updated"]
    for oo in x:
        name = oo["updated"]["name"]
        oo["updated"]["name"] = nid.join(name.split("DUMMYNID"))
        oo["updated"]["new"] = oid
    o["updated"] = x

with open(filename, "w") as f:
    json.dump(o, fp=f, indent=4)
~~~

## Set environment variables and run command

To avoid having to repeat environment variables to set up and use a
Radicle node for verification scenarios, we provide a script that
sets them and runs a command.

~~~{#radenv.sh .file .sh}
#!/bin/bash

set -euo pipefail

homedir="$(pwd)/homedir"

env \
    SRCDIR="$CARGO_MANIFEST_DIR" \
    PATH=~/.radicle/bin:"$PATH" \
	HOME="$homedir" \
	RAD_PASSPHRASE=secret \
	RAD_HOME="$homedir/.radicle" \
	RAD_SOCKET=synt.sock \
	"$@"
~~~


## Set up a node with repository

Most of our verification scenarios will need to set up a Radicle node
and a test repository there. Rather than repeat it in every scenario,
we use this helper script.

~~~{#setup-node.sh .file .sh}
#!/bin/bash

set -xeuo pipefail

mkdir -p "$HOME"

rad auth --alias brokertest

git config --global user.email radicle@example.com
git config --global user.name TestyMcTestFace
git init -b main testy

cd testy
echo "test file" > test.txt

git add .
git commit -am test
git status
git show
rad init --name testy --description test --default-branch main --private --no-confirm --no-seed
rad inspect --identity
rad id list
~~~

# Acceptance criteria

## Shows config as JSON

_Want:_ The CI broker can write out the configuration it uses
at run time as JSON.

_Why:_ This is helpful for the node operator to verify that
they have configured the program correctly.

_Who:_ `cib-devs`

Our verification here is quite simplistic, and only checks that the
output is in the JSON format. It does not try to make sure the JSON
matches the YAML semantically.

~~~scenario
given an installed cib
given file broker.yaml
when I run cib --config broker.yaml config --output actual.json
when I run jq . actual.json
then command is successful
~~~


## Smoke test: Runs adapter

_Want:_ CI broker can run its adapter.

_Why:_ This is obviously necessary. If this doesn't work,
nothing else has a hope of working.

_Who:_ `cib-devs`

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed synthetic-events
given file refsfetched.json
given file set-rid
when I run bash radenv.sh env HOME=../homedir python3 set-rid refsfetched.json testy
when I run synthetic-events synt.sock refsfetched.json --log log.txt

given an installed cib
given a directory reports
given file broker.yaml
given file adapter.sh from dummy.sh
when I run chmod +x adapter.sh

when I run bash radenv.sh RAD_SOCKET=synt.sock cib --config broker.yaml process-events
then stderr contains "CI broker starts"
then stderr contains "loaded configuration"
then stderr contains "CI broker ends successfully"
then file reports/index.html exists
then file reports/status.json exists

given an installed cibtool
when I run cibtool --db ci-broker.db event list
then stdout is exactly ""

when I run cibtool --db ci-broker.db run list --json
then stdout contains ""id": "xyzzy""
~~~


## Reports it version

_Want:_ `cib` and `cibtool` report their version, if invoked with the
`--version` potion.

_Why:_ This helps node operators include the version in any bug
reports.

_Who:_ `cib-devs`

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cib
when I run cib --version
then stdout matches regex ^radicle-ci-broker \d+\.\d+\.\d+$

given an installed cibtool
when I run cibtool --version
then stdout matches regex ^radicle-ci-broker \d+\.\d+\.\d+$
~~~


## Adapter can provide URL for info on run

_Want:_ The adapter can provide a URL for information about the
run, such a run log. This optional.

_Why:_ The CI broker does not itself store the run log, but
it's useful to be able to point users at one. The CI broker can put
that into a Radicle COB or otherwise store it so that users can see
it. Note, however, that the adapter gets to decide which URL to
provide: it need not be the run log. It might, for example, be a URL
to the web view of a "pipeline" in GitLab CI instead, from which the
user can access individual logs.

_Who:_ `cib-devs`

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed synthetic-events
given file refsfetched.json
given file set-rid
when I run bash radenv.sh env HOME=../homedir python3 set-rid refsfetched.json testy
when I run synthetic-events synt.sock refsfetched.json --log log.txt

given file adapter.sh from adapter-with-url.sh
when I run chmod +x adapter.sh

given an installed cib
given file broker.yaml
given a directory reports

when I try to run bash radenv.sh RAD_SOCKET=synt.sock cib --config broker.yaml insert
then command is successful
~~~

~~~{#adapter-with-url.sh .file .sh}
#!/bin/bash
set -euo pipefail
echo '{"response":"triggered","run_id":{"id":"xyzzy"},"info_url":"https://ci.example.com/xyzzy"}'
echo '{"response":"finished","result":"success"}'
~~~

## Gives helpful error message if node socket can't be found

_Want:_ If the CI broker can't connect to the Radicle node
control socket, it gives an error message that helps the user to
understand the problem.

_Why:_ This helps users deal with problems themselves and
reduces the support burden on the Radicle project.

_Who:_ `cib-devs`

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cib
given file broker.yaml
when I try to run bash radenv.sh RAD_SOCKET=xyzzy.sock cib --config broker.yaml insert
then command fails
then stderr contains "node control socket does not exist: xyzzy.sock"
~~~


## Gives helpful error message if it doesn't understand its configuration file

_Want:_ If the CI broker is given a configuration file that it
can't understand, it gives an error message that explains the problem
to the user.

_Why:_ This helps users deal with problems themselves and
reduces the support burden on the Radicle project.

_Who:_ `cib-devs`

_Comment:_ This is a very basic scenario. Error handling is by nature
a thing that can always be made better. We can later add more
scenarios if we tighten the acceptance criteria.

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cib
given file broker.yaml
given file not-yaml.yaml
when I try to run env HOME=homedir cib --config not-yaml.yaml config
then command fails
then stderr contains "failed to parse configuration file as YAML: not-yaml.yaml"
~~~


~~~{#not-yaml.yaml .file}
This file is not YAML.
~~~

## Stops if the node connection breaks

_Want:_ If the connection to the Radicle node, via its control
socket, breaks, the CI broker terminates with a message saying why.

_Why:_ The CI broker can either keep running and trying to
re-connect, or it can terminate. Either is workable. However, it's a
simpler design and less code to terminate and allow re-starting to be
handled by a dedicated system, such as `systemd`.

_Who:_ `cib-devs`

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cib
given an installed synthetic-events
when I run synthetic-events synt.sock --log log.txt
given file broker.yaml
when I try to run bash radenv.sh RAD_SOCKET=synt.sock cib --config broker.yaml insert
then command is successful
~~~


## Shuts down when requested

_Want:_ The test suite can request the CI broker to shut down
cleanly, and it doesn't result in an error.

_Why:_ In the integration test suite, we need to start and
stop the CI broker many times. We need to easily detect errors.

_Who:_ `cib-devs`

We use a special magic fake node event to signal shutdown: a
`RefsFetched` event with a skipped update for a ref "`shutdown`" and
an object id of all zeros. This should be sufficiently impossible to
happen in real life.

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cib
given an installed synthetic-events
given file broker.yaml
when I run synthetic-events synt.sock --log synt.log
when I try to run bash radenv.sh RAD_SOCKET=synt.sock cib --config broker.yaml insert
when I run cat synt.log
then command is successful
when I run cibtool --db ci-broker.db run list
then stdout is exactly ""
~~~


## Produces a report page upon request

_Want:_ The node operator can run a command to produce a report
of all CI runs a CI broker instance has performed.

_Why:_ This is useful for diagnosis, if nothing else.

_Who:_ `cib-devs`

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cibtool
when I run bash radenv.sh cibtool --db x.db run add --repo testy --branch main --commit HEAD --failure

given a directory reports
when I run bash radenv.sh cibtool --db x.db report --output-dir reports

then file reports/index.html exists
then file reports/index.html contains "testy"
~~~

This doesn't check that there is a per-repository HTML file, because
we have not convenient way to know the repository ID.

## Logs adapter stderr output

_What:_ The CI broker should log, to its own log output, the adapter's
stderr output.

_Why:_ This allows the adapter to output its own log to its standard
error output. This makes it easier to debug adapter problems.

_Who:_ `adapter-devs`, `node-ops`

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cibtool
when I run bash radenv.sh cibtool --db ci-broker.db trigger --repo testy --ref main --commit HEAD
when I run bash radenv.sh cibtool --db ci-broker.db event list --json

given an installed cib
given file broker.yaml
given a directory reports
given file adapter.sh from dummy.sh
when I run chmod +x adapter.sh

when I run bash radenv.sh cib --config broker.yaml queued
then stderr contains "Mordor"
~~~

## Allows setting minimum log level

_What:_ The node admin should be able to set the minimum log level for
log messages that get output to stderr.

_Why:_ This allows controlling how much log spew log admins have to see.

_Who:_ `node-ops`

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cib
given file broker.yaml
given a directory reports

when I run bash radenv.sh cib --config broker.yaml config
then stderr contains "CI broker starts"

when I run bash radenv.sh cib --config broker.yaml --log-level error config
then stderr is exactly ""
~~~

# Acceptance criteria for test tooling

The event synthesizer is a helper to feed the CI broker node events in
a controlled fashion.

## We can run rad

_Want:_ We can run rad.

_Why:_ For many of the verification scenarios for the CI broker we
need to run the Radicle `rad` command line tool. Depending on the
environment we use for verification, `rad` may be installed in various
places. Commonly, if installed using the Radicle installer, `rad` is
installed into `~/.radicle/bin` and edits the shell initialization
file to add that to `$PATH`. However, in a CI context, that
initialization is not necessarily done and so the `radenv.sh` helper
script adds that directory to the `$PATH` just in case. We verify in
this scenario that we can run `rad` at all.

_Who:_ `cib-devs`

~~~scenario
given file radenv.sh
when I run bash radenv.sh rad --version
then command is successful
~~~


## Dummy adapter runs successfully

_Want:_ The dummy adapter (in embedded file `dummy.sh`) runs
successfully.

_Why:_ Test scenarios using the dummy adapter need to be
able to rely that it works.

_Who:_ `cib-devs`

~~~scenario
given file dummy.sh
when I run chmod +x dummy.sh
when I try to run ./dummy.sh
then command is successful
~~~

## Adapter with URL runs successfully

_Want:_ The adapter with a URL (in embedded file
`adapter-with-url.sh`) runs successfully.

_Why:_ Test scenarios using this adapter need to be able to
rely that it works.

_Who:_ `cib-devs`

~~~scenario
given file adapter-with-url.sh
when I run chmod +x adapter-with-url.sh
when I try to run ./adapter-with-url.sh
then command is successful
~~~

## Event synthesizer terminates after first connection

_Want:_ The event synthesizer runs in the background, but
terminates after the first connection.

_Why:_ This is needed so that it can be invoked in Subplot
scenarios.

_Who:_ `cib-devs`

We use the `synthetic-events --client` option to connect to the daemon
and wait for the daemon to delete the socket file. This is more
easily portable than using a generic tool such as `nc`, which has many
variants across operating systems.

We wait for up to ten seconds the `synthetic-events` daemon to remove
the socket file before we check that it's been deleted, but checking
for that every second. This avoids the trap of waiting for a fixed
time: if the time is too short, the scenario fails spuriously, and if
it's very long, the scenario takes longer than necessary.

~~~scenario
given an installed synthetic-events

then file synt.sock does not exist

when I run synthetic-events --log daemon.log synt.sock
then file synt.sock exists

when I run synthetic-events --client --log daemon.log synt.sock
then file synt.sock does not exist
~~~


# Acceptance criteria for persistent database

The CI broker uses an SQLite database for persistent data. Many
processes may need to access or modify the database at the same time.
While SQLite is good at managing that, it needs to be used in the
right way for everything to work correctly. The acceptance criteria in
this chapter address that.

To enable the verification of these acceptance criteria, the CI broker
database allows for a "counter", as a single row in a dedicated table.
Concurrency is tested by having multiple processes update the counter
at the same time and verifying the end result is as intended and that
every value is set exactly once.

## Count in a single process

_Want:_ A single process can increment the test counter
correctly.

_Why:_ If this doesn't work with a single process, it won't
work of multiple processes, either.

_Who:_ `cib-devs`

~~~scenario
given an installed cibtool
then file count.db does not exist
when I run cibtool --db count.db counter show
then stdout is exactly "0\n"
when I run cibtool --db count.db counter count --goal 1000
when I run cibtool --db count.db counter show
then stdout is exactly "1000\n"
~~~


## Insert events into queue

_Want:_ Insert broker events generated from node events into
persistent event queue in the database, when allowed by the CI broker
event filter.

_Why:_ This is fundamental for running CI when repositories
in a node change.

_Who:_ `cib-devs`

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cib
given an installed synthetic-events

given file refsfetched.json
when I run synthetic-events synt.sock refsfetched.json --log synt.log

given file broker.yaml
when I try to run bash radenv.sh env RAD_SOCKET=synt.sock cib --config broker.yaml insert
then command is successful

when I run cibtool --db ci-broker.db event list --json
then stdout contains "BranchUpdated"
then stdout contains ""branch": "main""
then stdout contains ""tip": "0000000000000000000000000000000000000000""
then stdout contains ""old_tip": "0000000000000000000000000000000000000000""
~~~

## Insert many events into queue

_Want:_ Insert many events that arrive quickly.

_Why:_ We need at least some rudimentary performance testing.

_Who:_ `cib-devs`

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cib
given an installed synthetic-events

given file refsfetched.json
when I run synthetic-events synt.sock refsfetched.json --log synt.log --repeat 1000

given file broker.yaml
when I try to run bash radenv.sh env RAD_SOCKET=synt.sock cib --config broker.yaml insert
then command is successful


when I run cibtool --db ci-broker.db event count
then stdout is exactly "1000\n"
~~~

## Don't insert events into queue when not allowed by filter

_Want:_ Nothing is inserted into the persistent event queue
then the CI broker's filter does not allow any events.

_Why:_ This is fundamental for running CI when repositories
in a node change.

_Who:_ `cib-devs`

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cib
given an installed synthetic-events

given file refsfetched.json
when I run synthetic-events synt.sock refsfetched.json --log synt.log

given file broker.yaml from broker-allow-nothing.yaml
when I try to run bash radenv.sh env RAD_SOCKET=synt.sock cib --config broker.yaml insert
then command is successful

when I run cibtool --db ci-broker.db event count
then stdout is exactly "0\n"
~~~

## Process queued events

_Want:_ It's possible to run the CI broker in a mode where it
only processes events from its persistent event queue.

_Why:_ This is primarily useful for testing the CI broker
queuing implementation.

_Who:_ `cib-devs`

We verify this by adding events to the queue with `cibtool`, and then
running the CI broker and verifying it terminates after processing the
events. We carefully add a shutdown event so that the CI broker shuts
down.

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given file adapter.sh from dummy.sh
when I run chmod +x adapter.sh

given an installed cib
given an installed cibtool

when I run bash radenv.sh cibtool --db ci-broker.db event add --repo testy --ref main --commit HEAD
when I run cibtool --db ci-broker.db event shutdown

given file broker.yaml
given a directory reports
when I run ls -l adapter.sh
when I run bash radenv.sh cib --config broker.yaml queued
then stderr contains "Action: run:"
then stderr contains "Action: shutdown"

when I run cibtool --db ci-broker.db event list
then stdout is exactly ""

when I run cibtool --db ci-broker.db run list --json
then stdout contains "success"
~~~


## Count in concurrent processes

_Want:_ Two process can concurrently increment the test counter
correctly.

_Why:_ This is necessary, if not necessarily sufficient, for
concurrent database use to work correctly.

_Who:_ `cib-devs`

Due to limitations in Subplot we mange the concurrent processes using
a helper shell script,k `count.sh`, found below. It runs two
concurrent `cibtool` processes that update the same database file, and
count to a desired goal. The script then verifies that everything went
correctly.

~~~scenario
given an installed cibtool
given file count.sh
when I run env bash -x count.sh 100 10
then stdout contains "OK\n"
~~~

~~~{#count.sh .file .sh}
#!/bin/bash

set -euo pipefail

run() {
	cibtool --db "$DB" counter count --goal "$goal"
}

DB=count.db

goal="$1"
reps="$2"

for x in $(seq "$reps"); do
	echo "Repetition $x"

	rm -f "$DB" ./?.out

	run >1.out 2>&1 &
	one=$!

	run >2.out 2>&1 &
	two=$!

	if ! wait "$one"; then
		echo "first run failed"
		cat 1.out
		exit 1
	fi

	if ! wait "$two"; then
		echo "second run failed"
		cat 2.out
		exit 1
	fi

	if grep ERROR ./?.out; then
		echo found ERRORs
		exit 1
	fi

	n="$(sqlite3 "$DB" 'select counter from counter_test')"
	[ "$n" == "$goal" ] || (
		echo "wrong count $n"
		exit 1
	)

	if awk '/increment to/ { print $NF }' ./?.out | sort -n | uniq -d | grep .; then
		echo "duplicate increments"
		exit 1
	fi
done

echo OK
~~~
# Acceptance criteria for management tool

The `cibtool` management tool can be used to examine and change the CI
broker database, and thus indirectly manage what the CI broker does.

## Events can be queued and removed from queue

_Want:_ `cibtool` can show the queued events, can inject an
event, and remove an event.

_Why:_ This is the minimum functionality needed to manage
the event queue.

_Who:_ `cib-devs`

We verify that this works by adding a new broker event, and then
removing it. We randomly choose the repository id for the CI broker
itself for this test, but the id shouldn't matter, it just needs to
be of the correct form.

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cibtool
when I run cibtool --db x.db event list
then stdout is exactly ""

when I run bash radenv.sh cibtool --db x.db event add --repo testy --ref main --commit HEAD --base HEAD --id-file id.txt

when I run cibtool --db x.db event show --id-file id.txt
then stdout contains "rad:"
then stdout contains "main"

when I run cibtool --db x.db event remove --id-file id.txt

when I run cibtool --db x.db event list
then stdout is exactly ""
~~~

## Can remove all queued events

_Want:_ `cibtool` can remove all queued events in one operation.

_Why:_ This will be useful if the CI broker changes how CI events or
their serialization in an incompatible way, again, or when the node
operator wants to prevent many CI runs from happening.

_Who:_ `cib-devs`, `node-ops`

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cibtool
when I run cibtool --db x.db event list
then stdout is exactly ""

when I run bash radenv.sh cibtool --db x.db event add --repo testy --ref main --commit HEAD --base HEAD
when I run bash radenv.sh cibtool --db x.db event add --repo testy --ref main --commit HEAD --base HEAD
when I run bash radenv.sh cibtool --db x.db event add --repo testy --ref main --commit HEAD --base HEAD
when I run bash radenv.sh cibtool --db x.db event add --repo testy --ref main --commit HEAD --base HEAD
when I run bash radenv.sh cibtool --db x.db event add --repo testy --ref main --commit HEAD --base HEAD
when I run bash radenv.sh cibtool --db x.db event add --repo testy --ref main --commit HEAD --base HEAD

when I run cibtool --db x.db event remove --all
when I run cibtool --db x.db event list
then stdout is exactly ""
~~~

## Can add shutdown event to queue

_Want:_ `cibtool` can add a shutdown event to the queued
events.

_Why:_ This is needed for testing, and for the node operator
to be able to do this cleanly.

_Who:_ `cib-devs`

~~~scenario
given an installed cibtool
when I run cibtool --db x.db event list
then stdout is exactly ""

when I run cibtool --db x.db event shutdown --id-file id.txt

when I run cibtool --db x.db event show --id-file id.txt
then stdout contains "Shutdown"
~~~
## Can trigger a CI run

_Want:_ The node operator can easily trigger a CI run without
changing the repository.

_Why:_ This allows running CI on a schedule, for example. It's also
useful for CI broker development.

_Who:_ `cib-devs`


~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cibtool
when I run bash radenv.sh cibtool --db x.db trigger --repo testy --ref main --commit HEAD --id-file id.txt

when I run cibtool --db x.db event show --id-file id.txt
then stdout contains "rad:"
then stdout contains "main"
~~~


## Add information about triggered run to database

_Want:_ `cibtool` can add information about a triggered CI run.

_Why:_ This is primarily needed for testing.

_Who:_ `cib-devs`

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cibtool
when I run cibtool --db x.db run list
then stdout is exactly ""

when I run bash radenv.sh cibtool --db x.db run add --id xyzzy --repo testy --branch main --commit main --triggered
when I run cibtool --db x.db run list --json
then stdout contains "xyzzy"
then stdout contains ""state": "triggered""
~~~

## Add information about run that's running to database

_Want:_ `cibtool` can add information about a CI run that's running.

_Why:_ This is primarily needed for testing.

_Who:_ `cib-dev`.

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cibtool
when I run cibtool --db x.db run list
then stdout is exactly ""

when I run bash radenv.sh cibtool --db x.db run add --repo testy --url https://x/1 --branch main --commit HEAD --running
when I run cibtool --db x.db run list --json
then stdout contains ""repo_name": "testy""
then stdout contains ""state": "running""
~~~


## Add information about run that's finished successfully to database

_Want:_ `cibtool` can add information about a CI run that's finished
successfully.

_Why:_ This is primarily needed for testing.

_Who:_ `cib-dev`.

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cibtool
when I run cibtool --db x.db run list
then stdout is exactly ""

when I run bash radenv.sh cibtool --db x.db run add --id xyzzy --repo testy --branch main --commit HEAD --success
when I run cibtool --db x.db run list --json
then stdout contains ""state": "finished""
then stdout contains ""result": "success""
then stdout contains "xyzzy"
~~~


## Add information about run that's finished in failure to database

_Want:_ `cibtool` can add information about a CI run that's failed.

_Why:_ This is primarily needed for testing.

_Who:_ `cib-dev`.

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cibtool
when I run cibtool --db x.db run list
then stdout is exactly ""

when I run bash radenv.sh cibtool --db x.db run add --id xyzzy --repo testy --branch main --commit HEAD --failure
when I run cibtool --db x.db run list --json
then stdout contains ""state": "finished""
then stdout contains ""result": "failure""
then stdout contains "xyzzy"

when I run cibtool --db x.db run list --adapter-run-id abracadabra
then stdout is exactly ""
~~~

## Update and show information about run to running

_Want:_ `cibtool` can update information about a CI run.

_Why:_ This is primarily needed for testing.

_Who:_ `cib-dev`.

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cibtool
when I run cibtool --db x.db run list
then stdout is exactly ""

when I run bash radenv.sh cibtool --db x.db run add --id x --repo testy --branch main --commit HEAD --triggered
when I run cibtool --db x.db run list
then stdout has one line
when I run cibtool --db x.db run show x
then stdout contains ""state": "triggered""
then stdout contains ""result": null"

when I run cibtool --db x.db run update --id x --running
when I run cibtool --db x.db run list
then stdout has one line
when I run cibtool --db x.db run show x
then stdout contains ""state": "running""
then stdout contains ""result": null"

when I run cibtool --db x.db run update --id x --success
when I run cibtool --db x.db run list
then stdout has one line
when I run cibtool --db x.db run show x
then stdout contains ""state": "finished""
then stdout contains ""result": "success""

when I run cibtool --db x.db run update --id x --failure
when I run cibtool --db x.db run list
then stdout has one line
when I run cibtool --db x.db run show x
then stdout contains ""state": "finished""
then stdout contains ""result": "failure""
~~~

## Don't insert event for non-existent repository

_Want:_ `cibtool` won't insert an event to the queue for a repository
that isn't in the local node.

_Why:_ This prevents adding events that can't ever trigger a CI run.

_Who:_ `cib-devs`

Note that we verify both lookup by name and by repository ID, and by
`cibtool event add` and `cibtool trigger`, to cover all the cases.

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cibtool

when I try to run bash radenv.sh cibtool --db x.db event add --repo missing --ref main --commit HEAD --base c0ffee
then command fails
then stderr contains "missing"

when I try to run bash radenv.sh cibtool --db x.db event add --repo rad:z3byzFpcfbMJBp4tKYyuuTZiP8WUB --ref main --commit HEAD --base c0ffee
then command fails
then stderr contains "rad:z3byzFpcfbMJBp4tKYyuuTZiP8WUB"

when I try to run bash radenv.sh cibtool --db x.db trigger --repo missing --ref main --commit HEAD --id-file id.txt
then command fails
then stderr contains "missing"

when I try to run bash radenv.sh cibtool --db x.db trigger --repo rad:z3byzFpcfbMJBp4tKYyuuTZiP8WUB --ref main --commit HEAD --id-file id.txt
then command fails
then stderr contains "rad:z3byzFpcfbMJBp4tKYyuuTZiP8WUB"

~~~

## Record node events

_What:_ Node operator can record node events into a file.

_Why:_ This can be helpful for remote debugging, it's very helpful for
CI broker development to see what events actually happen, and it's
useful for gathering data for trying out event filters.

_Who:_ `cib-devs`, `node-ops`

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed synthetic-events
given file refsfetched.json
given file set-rid
when I run bash radenv.sh env HOME=../homedir python3 set-rid refsfetched.json testy
when I run synthetic-events synt.sock refsfetched.json --log log.txt

given an installed cibtool
when I run bash radenv.sh cibtool event record --output events.json
then file events.json contains ""type":"refsFetched""
~~~

## Convert recorded node events into CI events

_What:_ Node operator can see what CI events are created from node
events.

_Why:_ This is helpful so that node operators can see what CI events
are created from node events, which may have been previously recorded.
It's also helpful for CI broker developers as a development tool.

_Who:_ `cib-dev`, `node-ops`

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed synthetic-events
given file refsfetched.json
given file set-rid
when I run bash radenv.sh env HOME=../homedir python3 set-rid refsfetched.json testy
when I run synthetic-events synt.sock refsfetched.json --log log.txt

given an installed cibtool
when I run bash radenv.sh cibtool event record --output node-events.json
when I run bash radenv.sh cibtool event ci --output ci-events.json node-events.json
when I run cat ci-events.json
then file ci-events.json contains "BranchUpdated""
~~~


## Filter recorded CI events

_What:_ Node operator can see what CI events an event filter allows.

_Why:_ This is helpful so that node operators can see verify their
event filters work as they expect.

_Who:_ `cib-dev`, `node-ops`

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed synthetic-events
given file refsfetched.json
given file set-rid
when I run bash radenv.sh env HOME=../homedir python3 set-rid refsfetched.json testy
when I run synthetic-events synt.sock refsfetched.json --log log.txt

given an installed cibtool
when I run bash radenv.sh cibtool event record --output node-events.json
when I run bash radenv.sh cibtool event ci --output ci-events.json node-events.json

given file allow.yaml
when I run cibtool event filter  allow.yaml ci-events.json
then stdout contains "BranchUpdated"

given file deny.yaml
when I run cibtool event filter deny.yaml ci-events.json
then stdout is exactly ""
~~~

~~~{#allow.yaml .file .yaml}
filters:
- !Branch "main"
~~~

~~~{#deny.yaml .file .yaml}
filters:
- !Branch "this-does-not-exist"
~~~

# Acceptance criteria for logging

The CI broker writes log messages to its standard error output
(stderr), which the node operator can capture to a suitable persistent
location. The logs are structured: each line is a JSON object. The
structured logs are meant to be easier to process by programs, for
example to extract information for monitoring, and alerting the node
operator about problems.

An example log message might look like below (here formatted on
multiple lines for human consumption):

~~~json
{
  "msg": "CI broker starts",
  "level": "INFO",
  "ts": "2024-08-14T13:38:36.733953135Z",
}
~~~

Because logs are crucial for managing a system, we record acceptance
criteria for the minimum logging that the CI broker needs to do.

## Logs start and successful end

_What:_ `cib` logs a message when it starts and ends.

_Why:_ The program starting to run can be important information, for
example, to know when it's not running. It's also important to know if
the CI broker terminates successfully.

_Who:_ `cib-dev`.

We verify this by starting `cib` in a mode where it processes any
events already in the event queue, and then terminates. We don't add
any events, so `cib` just terminates at once. All of this will work,
when properly set up.

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given file adapter.sh from dummy.sh
when I run chmod +x adapter.sh

given an installed cib

given file broker.yaml
given a directory reports
when I run bash radenv.sh cib --config broker.yaml queued

then stderr contains "CI broker starts"
then stderr contains "CI broker ends successfully"
~~~

## Logs termination due to error

_What:_ `cib` logs a message when it ends due to an unrecoverable
error.

_Why:_ It's quite important to know this. Note that a recoverable
error does not terminate the CI broker.

_Who:_ `cib-dev`.

We check this by running the CI broker without a local node. This is
an error it can't recover from.

~~~scenario
given an installed cib

given file broker.yaml
when I try to run env RAD_HOME=/does/not/exist cib --config broker.yaml queued

then stderr contains "CI broker starts"
then stderr contains "CI broker ends in unrecoverable error"
~~~

# Acceptance criteria for reports

The CI broker creates HTML and JSON reports on a schedule, as well as
when CI runs end. The scenarios in this chapter verify that those
reports are as wanted.

## Produces a JSON status file

_What:_ `cib` produces a JSON status file with information about the
current state of the CI broker.

_Why:_ This makes it easy to monitor the CI broker using an automated
monitoring system.

_Who:_ `node-ops`

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cib
given file broker.yaml
given a directory reports

when I run bash radenv.sh cib --config broker.yaml queued
then file reports/status.json exists

when I run jq .event_queue_length reports/status.json
then stdout is exactly "0\n"
~~~
# Acceptance criteria for upgrades

_What:_ The node operator can safely upgrade the CI broker. At the
very least, the CI broker developers need to know if they are making a
breaking change.

_Why:_ If software upgrades are tedious or risky, they happen less
often, to the detriment of everyone.

_Who:_ `cib-dev`, `node-ops`

It is important that those running the CI broker can upgrade
confidently. This requires, at least, that CI broker upgrades in
existing installations do not break anything, or at least not without
warning. The scenario in this chapter verifies that in a simple, even
simplistic manner.

Note that this upgrade testing is very much in its infancy. It is
expected to be fleshed out over time. There will probably be more
scenarios later.

The overall approach is as follows:

* we run various increasing versions of the CI broker
* we use the same configuration file and database for each version
* we have an isolated test node so that the CI broker can validate
  repository and commit
* for each version, we use `cibtool trigger` and `cib queued` to run
  CI
* after each version, we verify that the database has all the CI runs
  it had before running the version, plus one more

Note that because this scenario may be run outside the developer's
development environment, it is currently difficult to access the Git
tags that represent the CI broker releases. Thus we verify upgrades to
the Git commit identifiers instead. In any case, this scenario needs
to be updated when a new release is made.

~~~scenario
given file radenv.sh
given file setup-node.sh
when I run bash radenv.sh bash setup-node.sh

given an installed cibtool
given an installed cib
given file broker.yaml
given file verify-upgrade
given a directory reports

given file adapter.sh from dummy.sh
when I run chmod +x adapter.sh

when I touch file run-list.txt
when I run bash radenv.sh bash -x verify-upgrade run-list.txt 04ac3852a52a0256a126252b2c46dea225189441
when I run bash radenv.sh bash -x verify-upgrade run-list.txt 5ae4d012f2b2f3df6fbbfce9b7d739f13363d0ec
when I run bash radenv.sh bash -x verify-upgrade run-list.txt HEAD
~~~

~~~{#verify-upgrade .file .sh}
#!/bin/bash
#
# Given a list of CI runs and a CI broker version, build and run that
# version so that it triggers and runs CI on a given change. Then
# verify the CI broker database has the CI runs in the list, plus one
# more, and then update the list.

set -euo pipefail

REPO="testy"

LIST="$1"
VERSION="$2"

# Unset this so that the Cargo cache doesn't get messed up. (This
# smells like a caching bug, or my misundestanding.)
unset CARGO_TARGET_DIR

# Remember where various things are.
db="$(pwd)/ci-broker.db"
reports="$(pwd)/reports"
adapter="$(pwd)/adapter.sh"

# Remember where the config is and update config to use correct
# database and report directory.
config="$(pwd)/broker.yaml"
sed -i "s,^db:.*,db: $db," "$config"
sed -i "s,^report_dir:.*,report_dir: $reports," "$config"
sed -i "s,command:.*,command: $adapter," "$config"
nl "$config"


# Get source code for CI broker. The scenario that uses this script
# set $SRCDIR to point at the source tree, so we get the source code
# from there to avoid having to fetch things from the network.
rm -rf ci-broker html
mkdir ci-broker html
(cd "$SRCDIR" && git archive "$VERSION") | tar -C ci-broker -xf -

# Do things in the exported CI broker source tree. Capture stdout to a
# new list of CI run.
(
	cd ci-broker

	# Build source code.
	cargo build --all-targets -q

    (echo "Old CI run lists:"
	cargo run -q --bin cibtool -- --db "$db" run list 1>&2
	cargo run -q --bin cibtool -- --db "$db" run list --json) 1>&2

	# Trigger a CI run. Hide the event ID that cibtool writes to
    # stdout.
	cargo run -q --bin cibtool -- --db "$db" trigger --repo "$REPO" --name main --commit HEAD >/dev/null

	# Run CI on queued events.
	cargo run -q --bin cib -- --config "$config" queued

	# List CI runs now in database.
	cargo run -q --bin cibtool -- --db "$db" run list
) >"$LIST.new"

# Check that new list contains everything in old list, plus one more.
removed="$(diff -u <(sort "$LIST") <(sort "$LIST.new") | sed '1,/^@@/d' | grep -c "^-" || true)"
added="$(diff -u <(sort "$LIST") <(sort "$LIST.new") | sed '1,/^@@/d' | grep -c "^+" || true)"

if [ "$removed" = 0 ] && [ "$added" = 1 ]; then
	echo "CI broker $VERSION ran OK"
    mv "$LIST.new" "$LIST"
else
	echo "CI broker removed $removed, added $added CI runs." 1>&2
	exit 1
fi
~~~
