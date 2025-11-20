# Subplot -- acceptance criteria documentation and verification

Capture and communicate acceptance criteria for software and systems,
and how they are verified, in a way that's understood by all project
stakeholders.

Acceptance criteria are expressed as _scenarios_ in the Cucumber
given/when/then style:

> given a web site subplot.liw.fi
> when I retrieve the site front page
> then it contains "Subplot"
> and it contains "acceptance criteria"


## When all stakeholders really need to understand acceptance criteria

Subplot is a set of tools for specifying, documenting, and
implementing automated acceptance tests for systems and software.
Subplot tools aim to produce a human-readable document of acceptance
criteria and a program that automatically tests a system against those
criteria. The goal is for every stakeholder in a project to understand
the project’s acceptance criteria and how they’re verified.

See <https://subplot.liw.fi/> for the home page.

## Hacking Subplot

Subplot is written using the Rust programming language, so the usual
workflow for Rust applies.

* To build: `cargo build`
* To run tests: `cargo test`
* To format code: `cargo fmt`

You probably need to install Rust using rustup: the version packaged
in a Linux distribution is likely too old. When you install Rust,
ensure you have the following installed:

* `rustc`
* `cargo`
* `rustfmt`

To run the whole test suite, including testing all examples and
Subplot self tests, run `./check` at the root of the source tree.

You'll need to install some build dependencies. On a system running
Debian or a derivative of it:

~~~sh
$ sudo apt-get install build-essential git debhelper python3 \
  librsvg2-bin graphviz plantuml daemonize procps
~~~

Additionally, any packages reported by running the following command:

~~~sh
$ dpkg-checkbuilddeps
~~~

To build the Debian package:

~~~sh
$ git archive HEAD | gzip > "../subplot_$(dpkg-parsechangelog -SVersion | sed 's/-.*$//').orig.tar.gz"
$ dpkg-buildpackage -us -uc
~~~

# Legalese

The Subplot software is released using the MIT licence. The copy of
the licence text is from <https://mit-license.org/> originally.

## The MIT License (MIT)

Copyright 2019-2022  Lars Wirzenius, Daniel Silverstone

Permission is hereby granted, free of charge, to any person obtaining
a copy of this software and associated documentation files (the
“Software”), to deal in the Software without restriction, including
without limitation the rights to use, copy, modify, merge, publish,
distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so, subject to
the following conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

Fork this project to create your own MIT license that you can always
link to.
