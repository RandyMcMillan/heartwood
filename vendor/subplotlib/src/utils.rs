//! Utility functions used by subplotlib or the generated test functions

use base64::prelude::{Engine as _, BASE64_STANDARD};

/// Decode a base64 string.
///
/// If the result is not a valid utf8 string then it is lossily coerced.
///
/// ```
/// # use subplotlib::prelude::*;
///
/// assert_eq!(base64_decode("aGVsbG8="), "hello");
/// ```
///
/// # Panics
///
/// Will panic if it's not valid base64 leading to a string.
pub fn base64_decode(input: &str) -> String {
    let dec = BASE64_STANDARD.decode(input).expect("bad base64");
    String::from_utf8_lossy(&dec).to_string()
}
