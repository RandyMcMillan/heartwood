#![allow(dead_code)]
extern crate qcheck_macros;

use qcheck_macros::quickcheck;

fn reverse<T: Clone>(xs: &[T]) -> Vec<T> {
    let mut rev = vec![];
    for x in xs {
        rev.insert(0, x.clone())
    }
    rev
}

#[quickcheck]
fn double_reversal_is_identity(xs: Vec<isize>) -> bool {
    xs == reverse(&reverse(&xs))
}

fn main() {}
