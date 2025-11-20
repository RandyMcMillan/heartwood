#!/bin/bash

# Simple dictionary abstraction. All values are stored in files so
# they can more easily be inspected.

dict_new() {
    local name="$1"
    rm -rf "$name"
    mkdir "$name"
}

dict_has() {
    local name="$1"
    local key="$2"
    local f="$name/$key"
    test -e "$f"
}

dict_get() {
    local name="$1"
    local key="$2"
    local f="$name/$key"
    cat "$f"
}

dict_get_default() {
    local name="$1"
    local key="$2"
    local default="$3"
    local f="$name/$key"
    if [ -e "$f" ]
    then
        cat "$f"
    else
        echo "$default"
    fi
}

dict_set() {
    local name="$1"
    local key="$2"
    local value="$3"
    local f="$name/$key"
    echo "$value" > "$f"
}

dict_keys() {
    local name="$1"
    ls -1 "$name"
}
