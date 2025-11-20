---
title: Radicle CI architecture
author: The Radicle Team
...

# Overview

[Radicle](https://radicle.xyz/) is a peer-to-peer collaboration system
built on top of the git version control system. Radicle has support
for integrating with continuous integration (CI) systems, using an
architecture where a "broker" listens to events about changes to
repositories stored in a node, and launching the appropriate "adapter"
for each change, according to its configuration.

This means each node can opt into running CI for what projects and
changes according to the interests of the person whose node it is.

* The delegates for a repository might run CI on all patches to make
  merge decisions with more confidences.
* Someone whose contributing to a project might only care about
  patches they themselves created, and only run CI for those.
* A third party might run CI for projects they use, to know if it's OK
  to deploy to production.

Radicle provides its own, very simple "native CI" solution. It's just
good enough for the Radicle project to use itself. In addition, there
are adapters that integrate with external CI systems.

## Components in native CI

CI support in Radicle consists of several components. For native CI
they are:

* the Radicle node
* the CI broker
* the native CI executable

These all run on the same host.

![Components for native CI](comp.svg)

![Sequence diagram for native CI](architecture.svg)

## Components when integrating an external CI system

For external CI, the components are:

* the Radicle node
* the CI broker
* an adapter executable for each supported external CI instance
* the external CI instance

The first three of these run on the same host, but the external CI
instance can run anywhere. The adapter talks to the CI instance using
whatever protocol the CI instance supports, such as HTTP.

External CI integration works like this:

* a repository known to the node changes
  - a git ref is updated
  - the ref can be a branch, tag, or something else, such as a Radicle
    COB
  - the node emits an event describing the change
* the CI broker listens to node events
  - the broker subscribes to node events via the node control socket,
    which is a Unix domain socket
* the CI broker filters events, based on its configuration, and the
  configuration for the repository involved
* for an event that passes its filters, the CI broker spawns the
  appropriate adapter process
  - there are different adapter for different CI implementations
* the broker sends a request object to the adapter as a child process,
  via the child's stdin, and reads any responses from the child's
  stdout
  - the request is JSON
  - the responses are in the form of JSON Lines: a JSON object per
    line serialized to not contain newline characters
* the adapter communicates with the external CI instance in whatever
  way is suitable for that instance
  - this is usually over HTTP
  - it may involve the CI instance making a web hook request back to
    the adapter

![Components for external CI](comp-ext.svg)

![Sequence diagram for external CI](architecture-ext.svg)

# The adapter

The adapter process reads the request to perform a CI run on a
specific commit in a repository, and responds with the id of the run,
then later with the status of the finished run.

For native CI, the adapter actually is the CI engine and performs the
CI run itself. For external CI, the adapter process does whatever it
needs to do to get the external CI engine instance to perform the CI
run. If the CI engine calls back via web hook to notify of the run
finishing, the adapter process needs to receive the call and process
it.

External CI engines allow complex pipelines to be written and support 
a variety of workflows. Different jobs or tasks can be triggered
based on different events (e.g. `push`, `patch created`, `patch 
updated`, etc.), to satisfy different workflow needs. (Only a few of
these are actually implemented yet, but the scaffolding to support
more is there.)

Some examples of real-world use-cases:

- trigger "fast" tests on every push to any branch
- trigger "relatively fast" tests only when a Patch is created / 
updated
- trigger "full test suite" on every push to the default branch (e.g. 
`main`)

In order to allow developers using Radicle the same flexibility that
they are used to on other forges, we want the broker to pass on 
whatever information it already has from the node events to the 
adapters, so they can pass it on to external CI systems, as appropriate. 

# Request and response messages

Note: the JSON objects below are formatted on multiple lines to make
them easier to read. The actual wire format is one line per message.

Run the `broker-messages` binary to get actual examples produced by
code.

~~~{.sh .numberLines}
$ cargo run -q --bin broker-messages
Trigger request:
{"request":"trigger","event_type":"push","version":<PROTOCOL_VERSION>,repository":{"id":"rad:zwTxygwuz5LDGBq255RA2CbNGrz8","name":"radicle-ci-broker","description":"Radicle CI broker","private":false,"default_branch":"main","delegates":["did:key:z6MkgEMYod7Hxfy9qCvDv5hYHkZ4ciWmLFgfvm3Wn1b2w2FV"]},"pusher":{"id":"did:key:z6MkgEMYod7Hxfy9qCvDv5hYHkZ4ciWmLFgfvm3Wn1b2w2FV","alias":"liw"},"before":"b4fb1e347be7db19f0859717062f94116b5bec9f","after":"b4fb1e347be7db19f0859717062f94116b5bec9f","branch":"patches/8d8232ddcb217fa1402eec4d955e227ef3bb5881","commits":[]}

Triggered response:
{"response":"triggered","run_id":{"id":"any-string-works-as-run-id"}}

Successful response:
{"response":"finished","result":"success"}

Failure response:
{"response":"finished","result":"failure"}
~~~

## Push Event Request

An example request that the broker sends looks like this:

~~~{.json .numberLines}
{
    "request": "trigger",
    "event_type": "push",
    "version":<PROTOCOL_VERSION>,
    "pusher": {
        "id": "did:key:z6MkltRpzcq2ybm13yQpyre58JUeMvZY6toxoZVpLZ8YabRa",
        "alias": "node_alias"
    },
    "before": "<BEFORE_COMMIT>",
    "after": "<AFTER_COMMIT>",
    "branch": "<BRANCH_NAME>",
    "commits": [
        "<SOME_OTHER_COMMIT_BEING_PUSHED>",
        "<AFTER_COMMIT>"
    ],
    "repository": {
        "id": "<RID>",
        "name": "heartwood",
        "description": "Radicle is a sovereign peer-to-peer network for 
        code collaboration, built on top of Git.",
        "private": false,
        "default_branch": "main",
        "delegates": [
          "did:key:z6MkltRpzcq2ybm13yQpyre58JUeMvZY6toxoZVpLZ8YabRa",
          "did:key:z6MkltRpzcq2ybm13yQpyre58JUeMvZY6toxoZVpLZ8YabRb"
        ]
    }
}
~~~

where:

  - `<PROTOCOL_VERSION>` is the version of the protocol messages that broker exchanges
  - `<RID>` is the repository ID, in its `rad:` URN format,
  - `<BRANCH_NAME>` is the branch name where the push occurred,
  - `<AFTER_COMMIT>` is the commit id of the last commit being pushed,
  - `<BEFORE_COMMIT>` is the commit id of the **parent** of the first
     commit being pushed (i.e. ` <SOME_OTHER_COMMIT_BEING_PUSHED>`),
     (the SHA checksum).

The `request` fields allows us to extend this in the future.

## Patch Event Request

An example request that the broker sends looks like this:

~~~{.json .numberLines}
{
    "request": "trigger",
    "event_type": "patch",
    "version":<PROTOCOL_VERSION>
    "action": "created|updated",
    "patch": {
        "id": "<PATCH_ID>",
        "author": {
            "id": "did:key:z6MkltRpzcq2ybm13yQpyre58JUeMvZY6toxoZVpLZ8YabRa",
            "alias": "node_alias"
        },
        "title": "Add description in README",
        "state": {
            "status": "Open",
            "conflicts": [
                {
                    "revision_id": "string",
                    "oid": "string"
                }
            ]
        },
        "before": "<BEFORE_COMMIT>",
        "after": "<AFTER_COMMIT>",
        "commits": [
            "<SOME_OTHER_COMMIT_BEING_PUSHED>",
            "<AFTER_COMMIT>"
        ],
        "target": "delegates",
        "labels": [
            "small",
            "goodFirstIssue",
            "enhancement",
            "bug"
        ],
        "assignees": [
            "did:key:z6MkltRpzcq2ybm13yQpyre58JUeMvZY6toxoZVpLZ8YabRa"
        ],
        "revisions": [
            {
                "id": "41aafe22200464bf905b143d4233f7f1fa4a9123",
                "author": {
                    "id": "did:key:z6MkltRpzcq2ybm13yQpyre58JUeMvZY6toxoZVpLZ8YabRa",
                    "alias": "my_alias"
                },
                "description": "The revision description",
                "base": "193ed2f675ac6b0d1ab79ed65057c8a56a4fab23",
                "oid": "f0f5d38ffa8d54a7cc737fc4e75ab1e2e178eaa1",
                "timestamp": 1699437445
            }
        ]
    },
    "repository": {
        "id": "<RID>",
        "name": "heartwood",
        "description": "Radicle is a sovereign peer-to-peer network for 
        code collaboration, built on top of Git.",
        "private": false,
        "default_branch": "main",
        "delegates": [
          "did:key:z6MkltRpzcq2ybm13yQpyre58JUeMvZY6toxoZVpLZ8YabRa",
          "did:key:z6MkltRpzcq2ybm13yQpyre58JUeMvZY6toxoZVpLZ8YabRb"
        ]
    }
}
~~~

where:

  - `<PROTOCOL_VERSION>` is the version of the protocol messages that broker exchanges
  - `<RID>` is the repository ID, in its `rad:` URN format,
  - `<AFTER_COMMIT>` is the commit id of the last commit being pushed,
  - `<BEFORE_COMMIT>` is the commit id of the **parent** of the first
    commit being pushed (i.e. ` <SOME_OTHER_COMMIT_BEING_PUSHED>`),
    (the SHA checksum).

The `request` fields allows us to extend this in the future.

## Responses

The first response from the adapter looks like this:

~~~{.json .numberLines}
{
    "response": "triggered",
    "run_id": "<RUNID>"
}
~~~

where `<RUNID>` is the id of the run that has been triggered.

The second response from the adapter looks like this:

~~~{.json .numberLines}
{
    "response": "finished",
    "result": "<STATUS>"
}
~~~

where `<STATUS>` is either the string `success` or `failure`. Note
that the run id is not repeated as the context makes this clear: the
response comes from the same process, via the same stdout pipe, as the
previous message.

# Report generation

The CI broker has an SQLite database file for persistent storage of
information of the CI runs it triggers. This is used to generate
report pages.

The report pages are HTML, generated from the information in the
database. The broker loads information for all runs when it starts,
from the database, and then pushed information about new runs when
they happen. This avoids the broker having to read everything every
time it updates the report pages.

The report page generation is done in its own thread, separate from
the main thread of the CI broker. This allows the reporting to happens
independently of what the main thread is doing. In particular, it
means the main thread does not need to do anything to trigger reports
from being updated.

When it comes to per-run logs, for external CI these are kept by the
external CI instance and the broker never sees them. For native CI,
the native CI adapter writes them to the report directory, as
`$RUNID/log.html`, and the broker generates report pages that link to
those files.
