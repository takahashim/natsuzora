//! Include tests with real template files
//!
//! These tests verify include functionality using actual template files,
//! including multi-level nested includes.

use natsuzora::{Natsuzora, NatsuzoraError};
use serde_json::json;
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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
    let template_path = templates_dir().join(format!("{}.tmpl", name));
    let source = std::fs::read_to_string(&template_path).map_err(|e| NatsuzoraError::IoError(e))?;
    Natsuzora::parse_with_includes(&source, include_root())?.render(data)
}

fn render_source(source: &str, data: serde_json::Value) -> Result<String, NatsuzoraError> {
    Natsuzora::parse_with_includes(source, include_root())?.render(data)
}

mod basic_include {
    use super::*;

    #[test]
    fn includes_simple_partial() {
        let result = render_template("simple", json!({})).unwrap();
        assert_eq!(result.trim(), "Hello from simple partial");
    }

    #[test]
    fn includes_partial_with_argument() {
        let result = render_template("greeting", json!({"user": {"name": "Alice"}})).unwrap();
        assert_eq!(result.trim(), "Hello, Alice!");
    }

    #[test]
    fn includes_partial_directly() {
        let result = render_source("{[> /greeting name=name]}", json!({"name": "Bob"})).unwrap();
        assert_eq!(result.trim(), "Hello, Bob!");
    }
}

mod nested_path_includes {
    use super::*;

    #[test]
    fn includes_from_nested_directory() {
        let result = render_source(
            "{[> /components/card title=t body=b]}",
            json!({"t": "Title", "b": "Body text"}),
        )
        .unwrap();
        assert!(result.contains("<div class=\"card\">"));
        assert!(result.contains("<h2>Title</h2>"));
        assert!(result.contains("<p>Body text</p>"));
    }

    #[test]
    fn includes_button_component() {
        let result = render_source(
            "{[> /components/button className=cls label=lbl]}",
            json!({"cls": "btn-primary", "lbl": "Click me"}),
        )
        .unwrap();
        assert_eq!(result.trim(), "<button class=\"btn-primary\">Click me</button>");
    }
}

mod include_with_each_loop {
    use super::*;

    #[test]
    fn renders_multiple_cards() {
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
}

mod two_level_nested_includes {
    use super::*;

    #[test]
    fn renders_navigation_menu_with_items() {
        // /nav/menu includes /nav/item
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
}

mod three_level_nested_includes {
    use super::*;

    #[test]
    fn renders_full_page_layout_with_deep_nesting() {
        // /layout/page includes /layout/header, /layout/footer, /components/card
        // /layout/header includes /nav/menu
        // /nav/menu includes /nav/item
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

        // Check HTML structure
        assert!(result.contains("<!DOCTYPE html>"));
        assert!(result.contains("<title>Welcome - My Site</title>"));

        // Check header with nested nav
        assert!(result.contains("<header>"));
        assert!(result.contains("<h1>My Site</h1>"));
        assert!(result.contains("<nav>"));
        assert!(result.contains("<strong>Home</strong>"));
        assert!(result.contains("<a href=\"/blog\">Blog</a>"));

        // Check main content with cards
        assert!(result.contains("<main>"));
        assert!(result.contains("<h2>Welcome</h2>"));
        assert!(result.contains("<div class=\"cards\">"));
        assert!(result.contains("<h2>Feature 1</h2>"));
        assert!(result.contains("<h2>Feature 2</h2>"));

        // Check footer
        assert!(result.contains("<footer>"));
        assert!(result.contains("&copy; 2024 My Site"));
    }

    #[test]
    fn renders_page_without_cards_when_empty_array_provided() {
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
}

mod include_argument_shadowing {
    use super::*;

    #[test]
    fn allows_shadowing_in_include_scope() {
        // Create a template that shadows a variable name
        let result = render_source(
            "{[ name ]} -> {[> /greeting name=other]} -> {[ name ]}",
            json!({"name": "Original", "other": "Shadowed"}),
        )
        .unwrap();
        assert_eq!(result.trim(), "Original -> Hello, Shadowed!\n -> Original");
    }
}

mod include_with_path_arguments {
    use super::*;

    #[test]
    fn passes_nested_path_as_argument() {
        let result = render_source(
            "{[> /greeting name=user.profile.displayName]}",
            json!({"user": {"profile": {"displayName": "Charlie"}}}),
        )
        .unwrap();
        assert_eq!(result.trim(), "Hello, Charlie!");
    }
}

mod include_inside_conditional {
    use super::*;

    #[test]
    fn conditionally_includes_partial_when_true() {
        let result = render_source(
            "{[#if showGreeting]}{[> /greeting name=name]}{[/if]}",
            json!({"showGreeting": true, "name": "Dave"}),
        )
        .unwrap();
        assert_eq!(result.trim(), "Hello, Dave!");
    }

    #[test]
    fn conditionally_excludes_partial_when_false() {
        let result = render_source(
            "{[#if showGreeting]}{[> /greeting name=name]}{[/if]}",
            json!({"showGreeting": false, "name": "Dave"}),
        )
        .unwrap();
        assert_eq!(result, "");
    }
}

mod error_cases {
    use super::*;

    #[test]
    fn raises_error_for_missing_partial() {
        let result = render_source("{[> /nonexistent]}", json!({}));
        assert!(result.is_err());
        if let Err(NatsuzoraError::IncludeError { message }) = result {
            assert!(message.contains("not found"));
        } else {
            panic!("Expected IncludeError");
        }
    }

    #[test]
    fn raises_error_for_invalid_include_name_with_double_dot() {
        // Note: '..' in path causes parse error because '.' is not valid in include names
        let result = render_source("{[> /path/../traversal]}", json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn raises_error_for_include_name_with_double_slash() {
        // Note: '//' causes parse error at lexer level because second '/' is not followed by valid char
        let result = render_source("{[> /path//double]}", json!({}));
        assert!(result.is_err());
    }
}
