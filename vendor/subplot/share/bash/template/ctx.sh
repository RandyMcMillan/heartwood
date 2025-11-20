#!/bin/bash

# A context abstraction using dictionaries.

ctx_new() {
    dict_new _ctx
}

ctx_set()
{
    dict_set _ctx "$1" "$2"
}

ctx_get()
{
    dict_get _ctx "$1"
}
