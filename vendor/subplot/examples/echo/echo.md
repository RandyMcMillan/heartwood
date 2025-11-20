Introduction
=============================================================================

**echo**(1) is a Unix command line tool, which writes its command line
arguments to the standard output. This is a simple acceptance test
suite for the `echo` implementation.

For more information, see [@foo2020].

No arguments
=============================================================================

Run `echo` without arguments.

```scenario
when user runs echo without arguments
then exit code is 0
then standard output contains a newline
then standard error is empty
```

Hello, world
=============================================================================

This scenario runs `echo` to produce the output "hello, world".

```scenario
when user runs echo with arguments hello, world
then exit code is 0
then standard output contains "hello, world"
then standard error is empty
```


# References
