quickcheck
==========
QuickCheck is a way to do property based testing using randomly generated
input. This crate comes with the ability to randomly generate and shrink
integers, floats, tuples, booleans, lists, strings, options and results.
All QuickCheck needs is a property function—it will then randomly generate
inputs to that function and call the property for each set of inputs. If the
property fails (whether by a runtime error like index out-of-bounds or by not
satisfying your property), the inputs are "shrunk" to find a smaller
counter-example.

The shrinking strategies for lists and numbers use a binary search to cover
the input space quickly.

### Documentation

The API is fully documented:
[https://docs.rs/qcheck](https://docs.rs/qcheck).

### The `#[quickcheck]` attribute

To make it easier to write QuickCheck tests, the `#[quickcheck]` attribute
will convert a property function into a `#[test]` function.

To use the `#[quickcheck]` attribute, you must import the `quickcheck` macro
from the `quickcheck_macros` crate:

```rust
#[cfg(test)]
mod tests {
    fn reverse<T: Clone>(xs: &[T]) -> Vec<T> {
        let mut rev = vec!();
        for x in xs {
            rev.insert(0, x.clone())
        }
        rev
    }

    #[quickcheck]
    fn double_reversal_is_identity(xs: Vec<isize>) -> bool {
        xs == reverse(&reverse(&xs))
    }
}
```

### Shrinking

Shrinking is a crucial part of QuickCheck that simplifies counter-examples for
your properties automatically. For example, if you erroneously defined a
function for reversing vectors as: (my apologies for the contrived example)

```rust
fn reverse<T: Clone>(xs: &[T]) -> Vec<T> {
    let mut rev = vec![];
    for i in 1..xs.len() {
        rev.insert(0, xs[i].clone())
    }
    rev
}
```

And a property to test that `xs == reverse(reverse(xs))`:

```rust
fn prop(xs: Vec<isize>) -> bool {
    xs == reverse(&reverse(&xs))
}
quickcheck(prop as fn(Vec<isize>) -> bool);
```

Then without shrinking, you might get a counter-example like:

```
[quickcheck] TEST FAILED. Arguments: ([-17, 13, -12, 17, -8, -10, 15, -19,
-19, -9, 11, -5, 1, 19, -16, 6])
```

Which is pretty mysterious. But with shrinking enabled, you're nearly
guaranteed to get this counter-example every time:

```
[quickcheck] TEST FAILED. Arguments: ([0])
```

Which is going to be much easier to debug.

##### Cranking the Number of Tests

Another approach is to just ask quickcheck to run properties more
times. You can do this either via the
[tests()](https://docs.rs/quickcheck/*/quickcheck/struct.QuickCheck.html#method.tests)
method, or via the `QUICKCHECK_TESTS` environment variable.
This will cause quickcheck to run for a much longer time. Unlike,
the loop approach this will take a bounded amount of time, which
makes it more suitable for something like a release cycle that
wants to really hammer your software.

### Generating Structs

It is very simple to generate structs in QuickCheck. Consider the following
example, where the struct `Point` is defined:

```rust
struct Point {
    x: i32,
    y: i32,
}
```

In order to generate a random `Point` instance, you need to implement
the trait `Arbitrary` for the struct `Point`:

```rust
use qcheck::{Arbitrary, Gen};

impl Arbitrary for Point {
    fn arbitrary(g: &mut Gen) -> Point {
        Point {
            x: i32::arbitrary(g),
            y: i32::arbitrary(g),
        }
    }
}
```

## License

Dual-licensed under MIT or the [UNLICENSE](https://unlicense.org).
