//! Integration tests for include directive using shared fixture files.
//!
//! Mirrors Ruby's include_spec.rb, using the same fixture templates
//! from tests/fixtures/templates/.

use natsuzora::{render_with_includes, NatsuzoraError};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
}

fn templates_dir() -> PathBuf {
    fixtures_dir().join("templates")
}

fn include_root() -> PathBuf {
    templates_dir().join("shared")
}

fn render_template(name: &str, data: serde_json::Value) -> Result<String, NatsuzoraError> {
    let path = templates_dir().join(format!("{name}.ntzr"));
    let source =
        fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read {}", path.display()));
    render_with_includes(&source, data, include_root())
}

fn render_source(source: &str, data: serde_json::Value) -> Result<String, NatsuzoraError> {
    render_with_includes(source, data, include_root())
}

// ============================================================================
// Basic include
// ============================================================================

#[test]
fn include_simple_partial() {
    let result = render_template("simple", json!({})).unwrap();
    assert_eq!(result.trim(), "Hello from simple partial");
}

#[test]
fn include_partial_with_argument() {
    let result = render_template("greeting", json!({"user": {"name": "Alice"}})).unwrap();
    assert_eq!(result.trim(), "Hello, Alice!");
}

#[test]
fn include_partial_directly() {
    let result =
        render_source("{[!include /greeting name=name ]}", json!({"name": "Bob"})).unwrap();
    assert_eq!(result.trim(), "Hello, Bob!");
}

// ============================================================================
// Nested path includes
// ============================================================================

#[test]
fn include_from_nested_directory() {
    let result = render_source(
        "{[!include /components/card title=t body=b ]}",
        json!({"t": "Title", "b": "Body text"}),
    )
    .unwrap();
    assert!(result.contains("<div class=\"card\">"));
    assert!(result.contains("<h2>Title</h2>"));
    assert!(result.contains("<p>Body text</p>"));
}

#[test]
fn include_button_component() {
    let result = render_source(
        "{[!include /components/button className=cls label=lbl ]}",
        json!({"cls": "btn-primary", "lbl": "Click me"}),
    )
    .unwrap();
    assert_eq!(
        result.trim(),
        "<button class=\"btn-primary\">Click me</button>"
    );
}

// ============================================================================
// Include with each loop
// ============================================================================

#[test]
fn include_renders_multiple_cards() {
    let result = render_template(
        "card_list",
        json!({
            "cards": [
                {"title": "Card 1", "body": "Body 1"},
                {"title": "Card 2", "body": "Body 2"}
            ]
        }),
    )
    .unwrap();
    assert!(result.contains("<h2>Card 1</h2>"));
    assert!(result.contains("<h2>Card 2</h2>"));
    assert!(result.contains("<p>Body 1</p>"));
    assert!(result.contains("<p>Body 2</p>"));
    assert_eq!(result.matches("<div class=\"card\">").count(), 2);
}

// ============================================================================
// Two-level nested includes (/nav/menu includes /nav/item)
// ============================================================================

#[test]
fn include_two_level_navigation_menu() {
    let result = render_template(
        "nav_only",
        json!({
            "navItems": [
                {"label": "Home", "url": "/", "active": true},
                {"label": "About", "url": "/about", "active": false},
                {"label": "Contact", "url": "/contact", "active": false}
            ]
        }),
    )
    .unwrap();
    assert!(result.contains("<nav>"));
    assert!(result.contains("<ul>"));
    assert!(result.contains("<strong>Home</strong>"));
    assert!(result.contains("<a href=\"/about\">About</a>"));
    assert!(result.contains("<a href=\"/contact\">Contact</a>"));
    assert_eq!(result.matches("<li>").count(), 3);
}

// ============================================================================
// Three-level nested includes (layout → header → nav → item)
// ============================================================================

#[test]
fn include_three_level_full_page() {
    let result = render_template(
        "full_page",
        json!({
            "site": {
                "title": "My Site",
                "year": 2024,
                "nav": [
                    {"label": "Home", "url": "/", "active": true},
                    {"label": "Blog", "url": "/blog", "active": false}
                ]
            },
            "page": {
                "title": "Welcome",
                "cards": [
                    {"title": "Feature 1", "body": "Description 1"},
                    {"title": "Feature 2", "body": "Description 2"}
                ]
            }
        }),
    )
    .unwrap();

    // HTML structure
    assert!(result.contains("<!DOCTYPE html>"));
    assert!(result.contains("<title>Welcome - My Site</title>"));

    // Header with nested nav
    assert!(result.contains("<header>"));
    assert!(result.contains("<h1>My Site</h1>"));
    assert!(result.contains("<nav>"));
    assert!(result.contains("<strong>Home</strong>"));
    assert!(result.contains("<a href=\"/blog\">Blog</a>"));

    // Main content with cards
    assert!(result.contains("<main>"));
    assert!(result.contains("<h2>Welcome</h2>"));
    assert!(result.contains("<div class=\"cards\">"));
    assert!(result.contains("<h2>Feature 1</h2>"));
    assert!(result.contains("<h2>Feature 2</h2>"));

    // Footer
    assert!(result.contains("<footer>"));
    assert!(result.contains("&copy; 2024 My Site"));
}

#[test]
fn include_full_page_empty_cards() {
    let result = render_template(
        "full_page",
        json!({
            "site": {
                "title": "My Site",
                "year": 2024,
                "nav": []
            },
            "page": {
                "title": "Empty Page",
                "cards": []
            }
        }),
    )
    .unwrap();

    assert!(result.contains("<title>Empty Page - My Site</title>"));
    assert!(!result.contains("<div class=\"cards\">"));
}

// ============================================================================
// Include argument shadowing
// ============================================================================

#[test]
fn include_allows_shadowing_in_scope() {
    let result = render_source(
        "{[ name ]} -> {[!include /greeting name=other ]} -> {[ name ]}",
        json!({"name": "Original", "other": "Shadowed"}),
    )
    .unwrap();
    assert_eq!(result.trim(), "Original -> Hello, Shadowed!\n -> Original");
}

// ============================================================================
// Include with path arguments
// ============================================================================

#[test]
fn include_with_nested_path_argument() {
    let result = render_source(
        "{[!include /greeting name=user.profile.displayName ]}",
        json!({"user": {"profile": {"displayName": "Charlie"}}}),
    )
    .unwrap();
    assert_eq!(result.trim(), "Hello, Charlie!");
}

// ============================================================================
// Include inside conditional
// ============================================================================

#[test]
fn include_inside_if_true() {
    let result = render_source(
        "{[#if showGreeting]}{[!include /greeting name=name ]}{[/if]}",
        json!({"showGreeting": true, "name": "Dave"}),
    )
    .unwrap();
    assert_eq!(result.trim(), "Hello, Dave!");
}

#[test]
fn include_inside_if_false() {
    let result = render_source(
        "{[#if showGreeting]}{[!include /greeting name=name ]}{[/if]}",
        json!({"showGreeting": false, "name": "Dave"}),
    )
    .unwrap();
    assert_eq!(result, "");
}

// ============================================================================
// Error cases
// ============================================================================

#[test]
fn include_missing_partial_error() {
    let result = render_source("{[!include /nonexistent ]}", json!({}));
    assert!(matches!(result, Err(NatsuzoraError::IncludeError { .. })));
    if let Err(NatsuzoraError::IncludeError { message }) = result {
        assert!(
            message.contains("not found"),
            "Expected 'not found' in: {message}"
        );
    }
}

#[test]
fn include_double_dot_path_error() {
    // '..' causes parse error because '.' is not valid in include names
    let result = render_source("{[!include /path/../traversal ]}", json!({}));
    assert!(result.is_err());
}

#[test]
fn include_double_slash_error() {
    // '//' causes parse error because second '/' is not followed by valid identifier
    let result = render_source("{[!include /path//double ]}", json!({}));
    assert!(result.is_err());
}
