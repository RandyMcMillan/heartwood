# Introduction

  - who is this manual for?
  - what is Subplot meant for?
  - who is Subplot meant for?
  - history of Subplot
  - public use cases of Subplot

# Terminology

In Subplot we use the following terminology:

* A **need** (or "want" or "ask" or "request"): what a _stakeholder_
  wants from the system at the level they are concerned. This is often
  fuzzy, vague, and unclear and there may not be any way to verify
  that is is met.

  This is what a client tells a salesperson during the sales process,
  or an open source user says in a wishlist bug report.
  
  For example: a client might ask for the backup program to
  de-duplicate data in the backups; this is too vague to be
  implemented with any confidence. It leaves open too many details:
  what kind of data should be de-duplicated? At what granularity?
  Should it happen for data across users, and if so, how much can
  users know about each others' data? Should the de-duplication happen
  on the client machines, on the server, or only in storage?

* **Acceptance criteria:** needs expressed in a way that are clear,
  unambiguous, and specific, and with a specified way for verifying
  the criteria are met.

  This is what is nailed down in the contract so that it's clear when
  payment is due; or what an open source project commits to
  supporting.

* **Stakeholder:** anyone whose opinion about the system matters and
  who gets to affect what the acceptance criteria are, for their level of
  involvement. Stakeholders can be involved at various levels in a
  project. While the levels will vary between projects, they might be
  something like:
  
  * very high level: I am the CEO of the client, and I have final say
    on what we're willing to pay for, and I set the acceptance
    criteria that the system can back up all our important files on
    production systems; this will be verified by backing up all our
    production data.

  * medium level: I am the CTO of the client and I add the following
    acceptance criteria:
    
    - backed up data can be restored, verified by restoring a backup
      and comparing to the original data
    - backups are encrypted, verified by checking the backup for
      cleartext data
    - a full backup and restore of a terabyte of data takes at most
      one hour in a setup similar to our intended production systems,
      verified by measuring the backup and restore of production data

  * low level: I am the security analyst at the client and I add the
    acceptance criteria that the encryption uses SHA3 for integrity
    checking and the Ed25519 elliptic curve for encryption and digital
    signatures; these will be verified by third party inspection

* **Requirements:** since acceptance criteria are about the
  stakeholders accepting the software that gets offered to them, the
  development organization may use these criteria as a basis from
  which to derive more granular and specific statements about what the
  project should do and how to check that it does so. These are the
  traditional requirements and unit/integration tests which most
  software developers work with.
 

# An overview of acceptance criteria and their verification

  - discuss acceptance criteria vs requirements; functional vs
    non-functional requirements; automated vs manual testing
  - discuss stakeholders
  - discuss different approaches for verifying that a system meets it
    criteria
  - discuss how scenarios can be used to verify acceptance criteria

# Simple example project

  - discuss how to use Subplot for some simple, but not simplistic,
    software project
  - discuss different kinds of stakeholders a project may have

# Authoring Subplot documents

  - discuss that it may be necessary to have several documents for
    different audiences, at different levels of abstraction (cf. the
    FOSDEM safety devroom talk)
  - discuss writing style to target all the different stakeholders
  - discuss mechanics and syntax
    - Markdown and supported features
    - scenarios, embedded files, examples
    - bindings
    - step implementations in various languages
    - embedded markup for diagrams
    - running docgen

# Extended example project

  - discuss how to use Subplot for a significant project, but keep it
    sufficiently high level that it doesn't get too long and tedious
    to read

# Appendix: Implementing scenario steps in Bash

  - this appendix will explain how to implement scenario steps using
    the Bash shell

# Appendix: Implementing scenario steps in Python

  - this appendix will explain how to implement scenario steps using
    the Python language

# Appendix: Implementing scenario steps in Rust

  - this appendix will explain how to implement scenario steps using
    the Rust language


# Appendix: Scenario

This is currently necessary so that codegen won't barf.

~~~scenario
when I run true
~~~
