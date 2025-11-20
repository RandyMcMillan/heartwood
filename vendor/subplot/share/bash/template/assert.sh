#!/bin/bash

# Check two values for equality and give an error if they are not equal
assert_eq() {
    if ! diff -u <(echo "$1") <(echo "$2")
    then
        echo "expected values to be identical, but they're not"
        return 1
    fi
}

# Check first value contains second value.
assert_contains() {
    if ! echo "$1" | grep -F "$2" > /dev/null
    then
        echo "expected first value to contain second value"
        return 1
    fi
}
