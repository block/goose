//! Tests for fuzzy matching utilities in text_editor.rs

use crate::developer::text_editor::{normalize_for_fuzzy_match, normalize_to_lf, strip_bom};

#[test]
fn test_strip_bom_with_bom() {
    let content = "\u{FEFF}Hello, world!";
    let (bom, text) = strip_bom(content);
    assert_eq!(bom, "\u{FEFF}");
    assert_eq!(text, "Hello, world!");
}

#[test]
fn test_strip_bom_without_bom() {
    let content = "Hello, world!";
    let (bom, text) = strip_bom(content);
    assert_eq!(bom, "");
    assert_eq!(text, "Hello, world!");
}

#[test]
fn test_normalize_to_lf_crlf() {
    let content = "line1\r\nline2\r\nline3";
    let normalized = normalize_to_lf(content);
    assert_eq!(normalized, "line1\nline2\nline3");
}

#[test]
fn test_normalize_to_lf_cr() {
    let content = "line1\rline2\rline3";
    let normalized = normalize_to_lf(content);
    assert_eq!(normalized, "line1\nline2\nline3");
}

#[test]
fn test_normalize_to_lf_mixed() {
    let content = "line1\r\nline2\rline3\nline4";
    let normalized = normalize_to_lf(content);
    assert_eq!(normalized, "line1\nline2\nline3\nline4");
}

#[test]
fn test_normalize_for_fuzzy_match_trailing_whitespace() {
    let content = "line1   \nline2\t\nline3  ";
    let normalized = normalize_for_fuzzy_match(content);
    assert_eq!(normalized, "line1\nline2\nline3");
}

#[test]
fn test_normalize_for_fuzzy_match_smart_single_quotes() {
    // U+2018 LEFT SINGLE QUOTATION MARK, U+2019 RIGHT SINGLE QUOTATION MARK
    let content = "It\u{2018}s a \u{2019}test\u{2019}";
    let normalized = normalize_for_fuzzy_match(content);
    assert_eq!(normalized, "It's a 'test'");
}

#[test]
fn test_normalize_for_fuzzy_match_smart_double_quotes() {
    // U+201C LEFT DOUBLE QUOTATION MARK, U+201D RIGHT DOUBLE QUOTATION MARK
    let content = "\u{201C}Hello\u{201D} said the \u{201C}world\u{201D}";
    let normalized = normalize_for_fuzzy_match(content);
    assert_eq!(normalized, "\"Hello\" said the \"world\"");
}

#[test]
fn test_normalize_for_fuzzy_match_dashes() {
    // U+2013 EN DASH, U+2014 EM DASH, U+2212 MINUS SIGN
    let content = "a\u{2013}b\u{2014}c\u{2212}d";
    let normalized = normalize_for_fuzzy_match(content);
    assert_eq!(normalized, "a-b-c-d");
}

#[test]
fn test_normalize_for_fuzzy_match_special_spaces() {
    // U+00A0 NO-BREAK SPACE, U+2003 EM SPACE
    let content = "hello\u{00A0}world\u{2003}test";
    let normalized = normalize_for_fuzzy_match(content);
    assert_eq!(normalized, "hello world test");
}

#[test]
fn test_normalize_for_fuzzy_match_combined() {
    // Test combining multiple normalizations
    let content = "\u{201C}It\u{2019}s\u{201D} a test\u{2014}really   \n  with spaces  ";
    let normalized = normalize_for_fuzzy_match(content);
    assert_eq!(normalized, "\"It's\" a test-really\n  with spaces");
}

#[test]
fn test_normalize_for_fuzzy_match_preserves_leading_whitespace() {
    // Leading whitespace should be preserved (only trailing is stripped)
    let content = "    indented line   \n  another line  ";
    let normalized = normalize_for_fuzzy_match(content);
    assert_eq!(normalized, "    indented line\n  another line");
}

#[test]
fn test_normalize_for_fuzzy_match_empty_string() {
    let content = "";
    let normalized = normalize_for_fuzzy_match(content);
    assert_eq!(normalized, "");
}

#[test]
fn test_normalize_for_fuzzy_match_only_whitespace() {
    let content = "   \n   \n   ";
    let normalized = normalize_for_fuzzy_match(content);
    assert_eq!(normalized, "\n\n");
}
