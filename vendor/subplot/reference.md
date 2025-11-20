# Introduction

This document describes how we guard against accidental breaking
changes in Subplot by running the current version against a curated
set of subplot documents.

# Subplot version 0.9.0

## Produce HTML page

~~~scenario
given an installed subplot
given a clone of https://gitlab.com/subplot/subplot.git in src at 5168420454b92205c13224a6801d3341d7f0c3d3
when I docgen subplot.subplot to test.html, in src
when I run, in src, subplot docgen subplot.subplot --merciful -o subplot.html -t python
then file src/test.html exists
~~~

## Generate and run test program

~~~scenario
given an installed subplot
given file run_test.sh
given a clone of https://gitlab.com/subplot/subplot.git in src at 5168420454b92205c13224a6801d3341d7f0c3d3
when I run, in src, subplot codegen subplot.subplot -o test-inner.py -t python
when I run bash run_test.sh
then command is successful
~~~

~~~{#run_test.sh .file .sh}
#!/bin/bash

set -euo pipefail

N=100

if python3 src/test-inner.py --log test-inner.log --env "PATH=$PATH" --env SUBPLOT_DIR=/
then
    exit=0
else
    exit="$?"
fi

if [ "$exit" != 0 ]
then
    # Failure. Show end of inner log file.

    echo "last $N lines of test-inner.log:"
    tail "-n$N" test-inner.log | sed 's/^/    /'
fi

exit "$exit"
~~~
