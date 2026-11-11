//! Tests for [`super::parse_jq_filter`] and the clap-flattened [`super::JsonOutput`].

use clap::Parser;

use super::*;

/// Tiny wrapper so we can exercise `JsonOutput` through clap in isolation.
#[derive(Debug, Parser)]
struct TestApp {
    #[clap(flatten)]
    json_filter: JsonOutput,
}

#[cfg(feature = "jq")]
#[test]
fn parse_jq_filter_accepts_simple_filter() {
    parse_jq_filter(".foo").expect("valid filter should compile");
}

#[cfg(feature = "jq")]
#[test]
fn parse_jq_filter_accepts_stdlib_functions() {
    // Exercises jaq-std defs/funs: .foo | length
    parse_jq_filter(".foo | length").expect("stdlib functions should compile");
}

#[cfg(feature = "jq")]
#[test]
fn parse_jq_filter_rejects_syntax_error() {
    let err = parse_jq_filter("@").expect_err("syntax error should be rejected");
    assert!(
        err.contains("`@`"),
        "error should quote the filter source with backticks, got: {err}"
    );
}

#[cfg(feature = "jq")]
#[test]
fn parse_jq_filter_rejects_empty_string() {
    let err = parse_jq_filter("").expect_err("empty filter should be rejected");
    assert!(
        err.contains("``"),
        "error should quote the (empty) filter source with backticks, got: {err}"
    );
}

#[cfg(feature = "jq")]
#[test]
fn parse_jq_filter_rejects_unknown_function() {
    let err = parse_jq_filter(".foo | bogus_function_name")
        .expect_err("unknown function should be rejected");
    assert!(
        err.contains("bogus_function_name") || err.contains("jq filter"),
        "error should mention the filter or the unknown name, got: {err}"
    );
}

#[cfg(feature = "jq")]
#[test]
fn clap_populates_filter_when_jq_is_provided() {
    let app = TestApp::try_parse_from(["test", "--jq", ".foo"]).expect("valid --jq parses");
    assert!(app.json_filter.filter.is_some());
}

#[test]
fn clap_filter_is_none_by_default() {
    let app = TestApp::try_parse_from(["test"]).expect("no --jq parses");
    assert!(app.json_filter.filter.is_none());
    assert!(!app.json_filter.force_json_output());
}

#[test]
fn clap_rejects_invalid_filter_at_parse_time() {
    // This is the core fail-fast invariant: an invalid filter fails during
    // clap parsing, not at runtime.
    let err = TestApp::try_parse_from(["test", "--jq", "@"])
        .expect_err("invalid --jq is rejected by clap");
    assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
}

#[cfg(not(feature = "jq"))]
#[test]
fn parse_jq_filter_without_engine_rejects_valid_and_invalid_expressions() {
    for source in [".", ".runs[] | .title", "@", ""] {
        let error = parse_jq_filter(source).expect_err("无引擎时不应编译过滤器");
        assert!(error.contains("未包含内置 jq 过滤器"));
    }
}

#[cfg(not(feature = "jq"))]
#[test]
fn clap_without_engine_rejects_jq_before_command_execution() {
    let error =
        TestApp::try_parse_from(["test", "--jq", ".runs"]).expect_err("有效表达式也必须明确拒绝");
    assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
    assert!(error.to_string().contains("未包含内置 jq 过滤器"));
}
