# Introduction

The Radicle CI broker runs CI for repositories in the local Radicle
node. This is the user guide for the CI broker.

The CI broker helps users run validation on changes to their software
project, by automating the building and testing of the projects when
anything in the repository changes. This is often called "continuous
integration".

(Technically, "continuous integration" is the software development
practice to merge changes into the main line of development
frequently, at least daily, to avoid painful merge conflict
resolutions. However, for this guide we say "CI" to mean "when
repository changes, perform these actions", which is a more generic,
and quite popular definition, if not very purist.)

# Overview

The Radicle node stores Git repositories and synchronizes them with
other Radicle nodes. The CI broker connects to its local node and gets
"node events" whenever anything changes in the node. The relevant
change for the CI broker is that a Git references ("refs") in a
repository have been created, updated, or deleted. For now, these are
branches. Later, Radicle and the CI broker will support other
references, such as tags.

There are no node events for Git repositories being created or
deleted. It's not possible to create a Radicle repository without
creating a branch, so just looking at references is enough.

The CI broker looks at the reference changes and refines them into "CI
events", which are more suitable for the kind CI use that the CI
broker is meant to enable, than "this ref changed", which is quite low
level.

# CI events

The CI broker currently supports a small set of CI events. There will
be more.

In the tables below, the fields have the following meanings:

* `from_node` -- the node from which the event originated
* `repo` -- the ID of the repository concerned
* `branch` -- the name of the branch created of updated
* `tip` -- the newest commit in the branch or patch
* `old_tip` -- the previous newest tip, before the change


## `BranchCreated`

A branch has been created. This may mean the repository has also been
created, but that is not certain.

| Event           | fields              | field types |
|:----------------|:--------------------|:------------|
| `BranchCreated` | `from_node`         | `NodeId`    |
|                 | `repo`              | `RepoId`    |
|                 | `branch`            | `RefSting`  |
|                 | `tip`               | `Oid`       |

## `BranchUpdated`

A branch has been updated.

| Event           | fields      | field types |
|:----------------|:------------|:------------|
| `BranchUpdated` | `from_node` | `NodeId`    |
|                 | `repo`      | `RepoId`    |
|                 | `branch`    | `RefSting`  |
|                 | `tip`       | `Oid`       |
|                 | `old_tip`   | `Oid`       |

## `BranchDeleted`

A branch has been deleted.

| Event           | fields      | field types |
|:----------------|:------------|:------------|
| `BranchDeleted` | `from_node` | `NodeId`    |
|                 | `repo`      | `RepoId`    |
|                 | `branch`    | `RefString` |
|                 | `tip`       | ` Oid`      |

## `PatchCreated`

A patch has been created.

| Event          | fields      | field types |
|:---------------|:------------|:------------|
| `PatchCreated` | `from_node` | `NodeId`    |
|                | `repo`      | `RepoId`    |
|                | `patch`     | `PatchId`   |
|                | `new_tip`   | `Oid`       |

## `PatchUpdated`

A patch has been updated.

| Event          | fields      | field types |
|:---------------|:------------|:------------|
| `PatchUpdated` | `from_node` | `NodeId`    |
|                | `repo`      | `RepoId`    |
|                | `patch`     | `PatchId`   |
|                | `new_tip`   | `Oid`       |
|                |             |             |

# Event filters

The CI broker configuration can use the following conditions, and
AND/OR/NOT operators to build a filter expression: if the expression
evaluates as "true", the event is allowed and will trigger a CI run.
Otherwise it is discarded and does not trigger a CI run.

| Condition       | Meaning                                                   |
|:----------------|:----------------------------------------------------------|
| `Node`          | Event originated from a specific node, identified by ID   |
| `Repository`    | Event refers to a specific repository, identified by ID   |
| `Branch`        | Event refers to a specific Git branch                     |
| `BranchCreated` | Branch was created                                        |
| `BranchUpdated` | Branch was updated                                        |
| `BranchDeleted` | Branch was deleted                                        |
| `Patch`         | Event refers to a specific patch, identified by ID        |
| `PatchCreated`  | Patch was created                                         |
| `PatchUpdated`  | Patch was updated                                         |
| `Allow`         | Change is allowed                                         |
| `Deny`          | Changes is not allowed                                    |
| `Not`           | Change is allowed is the operand expressions are is false |
| `And`           | Change is allowed if all the operands are true            |
| `Or`            | Change is allows if any of the operands is true           |

## Example

The following example is a snippet of YAML for the CI broker
configuration file to match events that refer to the` main` in the CI
broker repository.

~~~yaml
filters:
  - !And
    - !Repository "rad:zwTxygwuz5LDGBq255RA2CbNGrz8"
    - !Branch "main"
~~~

The conditions are expressed using the `!Foo` syntax in YAML. `Foo`
must be one of the operands from the table above. Simple values are
expressed as doubly quoted strings, and lists of operands are
sub-lists in YAML syntax.
