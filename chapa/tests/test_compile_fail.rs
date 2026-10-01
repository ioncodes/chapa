#[test]
fn invalid_declarations_and_forbidden_accessors() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
}
