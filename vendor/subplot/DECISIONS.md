# Big decisions made by the Subplot project

In the [2021-09-11 meeting][] we decided to start keeping a decision
log: a log of big decisions, often architectural ones. This file is
that log. Decisions should be discussed before being made so that
there is a strong consensus for them. Decisions can be changed or
overturned if they turn out to no longer be good. Overturning is a
decision.

[2021-09-11 meeting]: https://subplot.liw.fi/blog/2021/09/11/meeting/

Each decision should have its own heading. Newest decision should come
first. Updated or overturned decisions should have their section
updated to note their status, without moving them.

## Adopting a Developer Certificate of Origin

Date: 2023-10-07

What: The subplot project is adopting the use of DCO.

Why: To reduce the chance that, in the future when others might wish
to contribute to Subplot, anyone might claim that we have changes
which are not permitted, but without incurring the potential additional
complications associated with CLAs or the like.

Who: Daniel, Lars

Detail: A Developer Certificate of Origin is a statement saying that
the contributor has the right to make a contribution and to assign
permission to the project to thence redistribute it under the project
licence. You can read more at: <https://developercertificate.org/>
and at <https://en.wikipedia.org/wiki/Developer_Certificate_of_Origin>

This decision was taken during a Subplot project meeting and the
current scope of the decision is to document our use of DCO 1.1 in
the contribution guidelines, and to add to our merge request review
process a human-driven validation of the following of the DCO.

We leave open the possibility of enforcing DCO in some programmatic
way in the future.

We assert that the presence of a `Signed-off-by` footer in each git
commit in a merge request is the mechanism by which a developer signals
that the commit meets the assertions in the DCO 1.1.

## Breaking changes in Subplot

Date: 2023-08-27

What: Specify what is, and isn't, a breaking change in Subplot.

Why: We don't want to cause unnecessary work for users of Subplot, to
adapt to changes we make in Subplot. We want to be considerate and
also we don't want to scare off people who like Subplot, but don't
want to deal with a constant churn of busywork.

Who: Daniel, Lars

Details: Our main guideline should be that if an existing, working
subplot document or the code generated from it stops working after a
change, it was a breaking change. However, the stricter we are about
this, the harder it will be for us to make any changes, and thus we
think we should, at least in this stage of Subplot's development, be
more relaxed. We propose we keep two lists of criteria: one for things
we do consider to be breaking changes, and we don't.

We should consider changes from release to release, not just per
commit.

We need to consider:

* the command line interface
* the Rust libraries we provide
* the YAML metadata file
* the markdown files
* bindings files
* step implementation files
* the step libraries provided by Subplot: the steps and their
  implementation; this means the bindings files and the Python and
  Rust code for the step implementations, and the "context" used or
  modified by step implementations

For the "yes, these are likely to be breaking changes":

* Dropping or renaming, the name of the Subplot binary.
* Dropping or renaming a command line option or subcommand, or the
  arguments they take.
* Changing what output files are produced by the `subplot` command.
* Changing Subplot so that users would have to upgrade both
  `subplotlib` and `subplot-build` at the same time.
* Dropping or changing the meaning of a YAML metadata or bindings file
  key. Adding keys is not a breaking change (see below).
* Dropping support for or changing the meaning of a markdown feature.
* Dropping or renaming a step library.
* Dropping an implementation language for a step library.
* Dropping or renaming a step in a step library, or what captures it
  takes.
* Changing to behavior of a step, including how it uses or modifies
  the context, in a way that an existing use of Subplot breaks or
  changes behavior in an unwanted way.
* Changing the output of `docgen` in a way that affects the meaning of
  the document.
* Changing or removing any exported part of a `subplotlib` context object,
  or any of the documented portions of the Python or Bash contexts.

For the "sorry, these aren't breaking changes":

* Anything that doesn't affect existing uses of Subplot.
* Adding new steps to the step libraries provided by the Subplot
  project. The new step may clash with an existing use of Subplot, but
  try to avoid this by phrasing steps carefully. However, if we can't
  add new steps, it becomes impossible to improve the libraries.
* Typographical and document navigation changes in `docgen` output.
  This includes output formats, navigational elements, automatically
  generated "anchors" for document elements, and the way document
  elements are typeset.


## Output format for `docgen`

Date: 2023-03-21

What: `subplot docgen` will only support HTML as the output format. We
no longer support PDF.

Why: We want to drop support of Pandoc for parsing markdown input
files, and this is simpler to achieve if we don't need to support PDF
output, as it relieves us from the need to produce an abstract syntax
tree in the Pandoc representation. Producing PDF without a Pandoc AST
is trickier.

Who: Daniel, Lars

## Minimum Supported Rust Version

Date: 2023-03-21

What: We decided that Subplot would support an MSRV of the version of
Rust in the Debian "testing" branch. We can bump our explicit MSRV
version when Debian gets a new version.

Who: Daniel, Lars

## Threshold for refactoring

Date: 2022-10-23

What: Instead of trying to do a whole code base tidy up, we'll
continuously do smaller refactoring changes when we see something that
needs improvement.

## Do not clear/override all environment variables

Date: 2021-10-08

What: We decided that the environment being "clean" is more the
resposibility of the caller of the test suite than of the test
suite itself. As such, we have decided to only set SHELL, HOME,
and TMPDIR. Importantly we do not override PATH etc. so that
things will work on NixOS etc.

Who: Daniel, Lars.

## Start decision log

Date: 2021-09-11

What: We decided to start keeping a decision log for big, often
architectural decisions affecting the project as a whole.

Who: Daniel, Lars.
