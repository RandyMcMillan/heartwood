#!/bin/bash

_run()
{
    if "$@" < /dev/null > stdout 2> stderr
    then
        ctx_set exit 0
    else
        ctx_set exit "$?"
    fi
    ctx_set stdout "$(cat stdout)"
    ctx_set stderr "$(cat stderr)"
}

run_echo_without_args()
{
    _run echo
}

run_echo_with_args()
{
    args="$(cap_get args)"
    _run echo "$args"
}

exit_code_is()
{
    actual_exit="$(ctx_get exit)"
    wanted_exit="$(cap_get exit_code)"
    assert_eq "$actual_exit" "$wanted_exit"
}

stdout_is_a_newline()
{
    stdout="$(ctx_get stdout)"
    assert_eq "$stdout" "$(printf '\n')"
}

stdout_is_text()
{
    stdout="$(ctx_get stdout)"
    text="$(cap_get text)"
    assert_contains "$stdout" "$text"
}

stderr_is_empty()
{
    stderr="$(ctx_get stderr)"
    assert_eq "$stderr" ""
}
