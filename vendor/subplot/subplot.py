import json
import os
import shutil


# A shell script to run the subplot binary from the source directory's Rust
# build target directory, adding the -t option to use the source directory's
# template directory.
#
# This is a string template where the source directory is filled in when used.
wrapper = """\
#!/bin/sh
set -eu
exec '{srcdir}/target/debug/{prog}' --resources '{srcdir}/share' "$@"
"""


def install_subplot(ctx):
    runcmd_prepend_to_path = globals()["runcmd_prepend_to_path"]

    # We require the SUBPLOT_DIR environment variable to be set to the path to
    # the directory where the Subplot binaries are installed.

    bindir = os.environ.get("SUBPLOT_DIR")
    assert (
        bindir is not None
    ), "MUST set SUBPLOT_DIR to directory where Subplot binaries are installed"
    runcmd_prepend_to_path(ctx, dirname=bindir)


def uninstall_subplot(ctx):
    if "bin-dir" in ctx:
        bindir = ctx["bin-dir"]
        if os.path.exists(bindir):
            shutil.rmtree(bindir)


def scenario_was_run(ctx, name=None):
    runcmd_stdout_contains = globals()["runcmd_stdout_contains"]
    runcmd_stdout_contains(ctx, text="\nscenario: {}\n".format(name))


def scenario_was_not_run(ctx, name=None):
    runcmd_stdout_doesnt_contain = globals()["runcmd_stdout_doesnt_contain"]
    runcmd_stdout_doesnt_contain(ctx, text="\nscenario: {}\n".format(name))


def step_was_run(ctx, keyword=None, name=None):
    runcmd_stdout_contains = globals()["runcmd_stdout_contains"]
    runcmd_stdout_contains(ctx, text="\n  step: {} {}\n".format(keyword, name))


def step_was_run_and_then(ctx, keyword1=None, name1=None, keyword2=None, name2=None):
    runcmd_stdout_contains = globals()["runcmd_stdout_contains"]
    runcmd_stdout_contains(
        ctx,
        text="\n  step: {} {}\n  step: {} {}".format(keyword1, name1, keyword2, name2),
    )


def cleanup_was_run(ctx, keyword1=None, name1=None, keyword2=None, name2=None):
    runcmd_stdout_contains = globals()["runcmd_stdout_contains"]
    runcmd_stdout_contains(
        ctx,
        text="\n  cleanup: {} {}\n  cleanup: {} {}\n".format(
            keyword1, name1, keyword2, name2
        ),
    )


def cleanup_was_not_run(ctx, keyword=None, name=None):
    runcmd_stdout_doesnt_contain = globals()["runcmd_stdout_doesnt_contain"]
    runcmd_stdout_doesnt_contain(ctx, text="\n  cleanup: {} {}\n".format(keyword, name))


def json_output_matches_file(ctx, filename=None):
    assert_dict_eq = globals()["assert_dict_eq"]

    ns = ctx.declare("_runcmd")
    actual = json.loads(ns["stdout"])
    expected = json.load(open(filename))
    assert_dict_eq(actual, expected)


def file_ends_in_zero_newlines(ctx, filename=None):
    assert_ne = globals()["assert_ne"]

    content = open(filename, "r").read()
    assert_ne(content[-1], "\n")


def file_ends_in_one_newline(ctx, filename=None):
    assert_eq = globals()["assert_eq"]
    assert_ne = globals()["assert_ne"]

    content = open(filename, "r").read()
    assert_eq(content[-1], "\n")
    assert_ne(content[-2], "\n")


def file_ends_in_two_newlines(ctx, filename=None):
    assert_eq = globals()["assert_eq"]

    content = open(filename, "r").read()
    assert_eq(content[-2:], "\n\n")


def binary(basename):
    srcdir = globals()["srcdir"]
    return os.path.join(srcdir, "target", "debug", basename)


def do_nothing(ctx):
    pass
