//! Compile-fail tests using trybuild
//!
//! These tests verify that incorrect usage produces compile errors.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
