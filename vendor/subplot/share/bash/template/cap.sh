#!/bin/bash

# Store step captures for calling the corresponding functions.

cap_new() {
    dict_new _cap
}

cap_set()
{
    dict_set _cap "$1" "$2"
}

cap_get()
{
    dict_get _cap "$1"
}
