---
title: Testing the Rust crate "subplotlib"
bindings:
  - subplotlib.yaml
  - lib/datadir.yaml
impls:
  rust:
    - helpers/subplotlib_context.rs
    - helpers/subplotlib_impl.rs
...

# Rust library for assisting with running subplots

The `subplotlib` crate is a library which assists with running test suites
generated using the `rust` template from Subplot.

This document is intended to **test** `subplotlib`, not to provide instruction
on how to use it.

# Fundamentals

All fundamental keywords are properly supported in subplotlib and the rust
template.

```scenario
given a counter starting at 0
when the counter is incremented
then the counter is 1
when the counter is incremented
then the counter is 2
```

# Embedded files

You can have files embedded into your subplot tests with the rust codegen
as well.

```{#example.txt .file}
This data file will be embedded into the test suite
```

```scenario
given I have read the file example.txt into EXAMPLE
when I look at EXAMPLE
then I see "will be embedded"
```

# Data directory

There is a data directory, and for now this test will fail if there's not
at least 1 megabyte of space in the data directory

```scenario
given datadir has at least 1024000B of space
and datadir has at least 1M of space
```
