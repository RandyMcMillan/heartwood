#!/bin/bash

# Store files embedded in the markdown input.

files_new() {
    # shellcheck disable=SC2154
    rm -rf "$_datadir/_files"
    mkdir "$_datadir/_files"
}

files_set() {
    printf "%s" "$2" >"$_datadir/_files/$1"
}

files_get() {
    cat "$_datadir/_files/$1"
}


# Decode a base64 encoded string.

decode_base64() {
    echo "$1" | base64 -d
}
