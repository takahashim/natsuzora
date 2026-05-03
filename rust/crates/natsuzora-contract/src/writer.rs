//! Writer for Contract notation.
//!
//! Converts Contract types to human-readable notation.

use crate::parser::ContractFileWithDiff;
use crate::types::{Contract, ContractField, ContractModifier, DiffMarker, ScalarType, TypeDef};

/// Write a Contract to human-readable notation.
pub fn write(contract: &Contract) -> String {
    let mut output = String::new();
    write_contract(contract, &mut output, 0);
    output
}

fn write_contract(contract: &Contract, output: &mut String, indent: usize) {
    match contract {
        Contract::Any => {
            // Any is not directly representable, skip
        }
        Contract::Scalar {
            scalar_type,
            modifier,
        } => {
            write_scalar(*scalar_type, *modifier, output);
        }
        Contract::Object { properties, .. } => {
            for (name, value) in properties {
                write_indent(output, indent);
                output.push_str(name);
                write_field_value(value, output, indent);
                output.push('\n');
            }
        }
        Contract::Array { items } => {
            output.push_str("[]");
            match items.as_ref() {
                Contract::Object { properties, .. } if !properties.is_empty() => {
                    output.push_str("{\n");
                    write_contract(items, output, indent + 1);
                    write_indent(output, indent);
                    output.push('}');
                }
                Contract::Scalar {
                    scalar_type,
                    modifier,
                } => {
                    write_scalar(*scalar_type, *modifier, output);
                }
                Contract::TypeRef { name } => {
                    output.push_str(name);
                }
                _ => {
                    output.push_str("scalar");
                }
            }
        }
        Contract::TypeRef { name } => {
            output.push_str(name);
        }
    }
}

fn write_field_value(contract: &Contract, output: &mut String, indent: usize) {
    match contract {
        Contract::Any => {
            output.push_str(": scalar");
        }
        Contract::Scalar {
            scalar_type,
            modifier,
        } => {
            output.push_str(": ");
            write_scalar(*scalar_type, *modifier, output);
        }
        Contract::Object { properties, .. } if !properties.is_empty() => {
            output.push_str(" {\n");
            write_contract(contract, output, indent + 1);
            write_indent(output, indent);
            output.push('}');
        }
        Contract::Object { .. } => {
            output.push_str(" {}");
        }
        Contract::Array { items } => {
            output.push_str(": []");
            match items.as_ref() {
                Contract::Object { properties, .. } if !properties.is_empty() => {
                    output.push_str("{\n");
                    write_contract(items, output, indent + 1);
                    write_indent(output, indent);
                    output.push('}');
                }
                Contract::Scalar {
                    scalar_type,
                    modifier,
                } => {
                    write_scalar(*scalar_type, *modifier, output);
                }
                Contract::TypeRef { name } => {
                    output.push_str(name);
                }
                _ => {
                    output.push_str("scalar");
                }
            }
        }
        Contract::TypeRef { name } => {
            output.push_str(": ");
            output.push_str(name);
        }
    }
}

fn write_scalar(scalar_type: ScalarType, modifier: ContractModifier, output: &mut String) {
    let type_name = match scalar_type {
        ScalarType::String => "string",
        ScalarType::Integer => "integer",
        ScalarType::Bool => "bool",
        ScalarType::Scalar => "scalar",
    };
    output.push_str(type_name);

    match modifier {
        ContractModifier::None => {}
        ContractModifier::Nullable => output.push('?'),
        ContractModifier::Required => output.push('!'),
    }
}

fn write_indent(output: &mut String, level: usize) {
    for _ in 0..level {
        output.push_str("  ");
    }
}

/// Write a ContractFileWithDiff to human-readable notation with diff markers.
pub fn write_with_diff(file: &ContractFileWithDiff) -> String {
    let mut output = String::new();

    // Write type definitions first
    for (name, type_def) in &file.types {
        write_type_def(name, type_def, &mut output);
        output.push('\n');
    }

    // Add blank line between types and fields if both exist
    if !file.types.is_empty() && !file.fields.is_empty() {
        output.push('\n');
    }

    // Write root fields
    for (name, field) in &file.fields {
        write_field_with_diff(name, field, &mut output, 0);
        output.push('\n');
    }

    output
}

fn write_type_def(name: &str, type_def: &TypeDef, output: &mut String) {
    // Write marker if present
    if let Some(marker) = &type_def.marker {
        match marker {
            DiffMarker::Added => output.push_str("+ "),
            DiffMarker::Removed => output.push_str("- "),
            DiffMarker::Changed => {} // Changed is not allowed for type defs
        }
    }

    output.push_str("type ");
    output.push_str(name);
    output.push_str(" {\n");

    // Write type body (fields from the contract)
    if let Contract::Object { properties, .. } = &type_def.contract {
        for (field_name, field_contract) in properties {
            write_indent(output, 1);
            output.push_str(field_name);
            write_field_value(field_contract, output, 1);
            output.push('\n');
        }
    }

    output.push('}');
}

fn write_field_with_diff(name: &str, field: &ContractField, output: &mut String, indent: usize) {
    write_indent(output, indent);

    // Write marker if present
    if let Some(marker) = &field.marker {
        match marker {
            DiffMarker::Added => output.push_str("+ "),
            DiffMarker::Removed => output.push_str("- "),
            DiffMarker::Changed => output.push_str("* "),
        }
    }

    output.push_str(name);

    // Handle changed fields (type -> type)
    if let Some(DiffMarker::Changed) = &field.marker {
        if let Some(next_type) = &field.next_type {
            output.push_str(": ");
            write_type_inline(&field.current_type, output);
            output.push_str(" -> ");
            write_type_inline(next_type, output);
            return;
        }
    }

    // Handle nested objects (inline block)
    match &field.current_type {
        Contract::Object { properties, .. } if !properties.is_empty() => {
            output.push_str(" {\n");
            for (field_name, field_contract) in properties {
                write_indent(output, indent + 1);
                output.push_str(field_name);
                write_field_value(field_contract, output, indent + 1);
                output.push('\n');
            }
            write_indent(output, indent);
            output.push('}');
        }
        _ => {
            write_field_value(&field.current_type, output, indent);
        }
    }
}

/// Write a type inline (for type change expressions like `integer -> scalar`).
fn write_type_inline(contract: &Contract, output: &mut String) {
    match contract {
        Contract::Any => {
            output.push_str("scalar");
        }
        Contract::Scalar {
            scalar_type,
            modifier,
        } => {
            write_scalar(*scalar_type, *modifier, output);
        }
        Contract::Array { items } => {
            output.push_str("[]");
            match items.as_ref() {
                Contract::Scalar {
                    scalar_type,
                    modifier,
                } => {
                    write_scalar(*scalar_type, *modifier, output);
                }
                Contract::TypeRef { name } => {
                    output.push_str(name);
                }
                _ => {
                    output.push_str("scalar");
                }
            }
        }
        Contract::TypeRef { name } => {
            output.push_str(name);
        }
        Contract::Object { .. } => {
            // For inline type display, objects are complex - simplify
            output.push_str("{}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn writes_simple_field() {
        let contract = Contract::Object {
            required: vec!["name".into()],
            properties: BTreeMap::from([(
                "name".into(),
                Contract::Scalar {
                    scalar_type: ScalarType::String,
                    modifier: ContractModifier::None,
                },
            )]),
        };

        let output = write(&contract);
        assert_eq!(output.trim(), "name: string");
    }

    #[test]
    fn writes_nullable_modifier() {
        let contract = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([(
                "name".into(),
                Contract::Scalar {
                    scalar_type: ScalarType::String,
                    modifier: ContractModifier::Nullable,
                },
            )]),
        };

        let output = write(&contract);
        assert_eq!(output.trim(), "name: string?");
    }

    #[test]
    fn writes_required_modifier() {
        let contract = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([(
                "name".into(),
                Contract::Scalar {
                    scalar_type: ScalarType::String,
                    modifier: ContractModifier::Required,
                },
            )]),
        };

        let output = write(&contract);
        assert_eq!(output.trim(), "name: string!");
    }

    #[test]
    fn writes_nested_object() {
        let contract = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([(
                "user".into(),
                Contract::Object {
                    required: vec![],
                    properties: BTreeMap::from([
                        (
                            "name".into(),
                            Contract::Scalar {
                                scalar_type: ScalarType::String,
                                modifier: ContractModifier::Required,
                            },
                        ),
                        (
                            "age".into(),
                            Contract::Scalar {
                                scalar_type: ScalarType::Integer,
                                modifier: ContractModifier::None,
                            },
                        ),
                    ]),
                },
            )]),
        };

        let output = write(&contract);
        assert!(output.contains("user {"));
        assert!(output.contains("name: string!"));
        assert!(output.contains("age: integer"));
    }

    #[test]
    fn writes_simple_array() {
        let contract = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([(
                "tags".into(),
                Contract::Array {
                    items: Box::new(Contract::Scalar {
                        scalar_type: ScalarType::String,
                        modifier: ContractModifier::None,
                    }),
                },
            )]),
        };

        let output = write(&contract);
        assert_eq!(output.trim(), "tags: []string");
    }

    #[test]
    fn writes_object_array() {
        let contract = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([(
                "items".into(),
                Contract::Array {
                    items: Box::new(Contract::Object {
                        required: vec![],
                        properties: BTreeMap::from([
                            (
                                "title".into(),
                                Contract::Scalar {
                                    scalar_type: ScalarType::String,
                                    modifier: ContractModifier::Required,
                                },
                            ),
                            (
                                "count".into(),
                                Contract::Scalar {
                                    scalar_type: ScalarType::Integer,
                                    modifier: ContractModifier::None,
                                },
                            ),
                        ]),
                    }),
                },
            )]),
        };

        let output = write(&contract);
        assert!(output.contains("items: []{"));
        assert!(output.contains("title: string!"));
        assert!(output.contains("count: integer"));
    }

    #[test]
    fn writes_bool_type() {
        let contract = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([(
                "isActive".into(),
                Contract::Scalar {
                    scalar_type: ScalarType::Bool,
                    modifier: ContractModifier::None,
                },
            )]),
        };

        let output = write(&contract);
        assert_eq!(output.trim(), "isActive: bool");
    }

    #[test]
    fn roundtrip_parse_write() {
        let input = r#"name: string!
age: integer
tags: []string
user {
  email: string?
  isAdmin: bool
}
items: []{
  count: integer
  title: string!
}
"#;

        let contract = crate::parse(input).unwrap();
        let output = write(&contract);
        let reparsed = crate::parse(&output).unwrap();

        // Verify structure matches
        match (&contract, &reparsed) {
            (
                Contract::Object {
                    properties: props1, ..
                },
                Contract::Object {
                    properties: props2, ..
                },
            ) => {
                assert_eq!(props1.len(), props2.len());
            }
            _ => panic!("expected objects"),
        }
    }

    #[test]
    fn writes_scalar_type() {
        let contract = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([(
                "value".into(),
                Contract::Scalar {
                    scalar_type: ScalarType::Scalar,
                    modifier: ContractModifier::None,
                },
            )]),
        };

        let output = write(&contract);
        assert_eq!(output.trim(), "value: scalar");
    }

    #[test]
    fn writes_integer_type() {
        let contract = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([(
                "count".into(),
                Contract::Scalar {
                    scalar_type: ScalarType::Integer,
                    modifier: ContractModifier::None,
                },
            )]),
        };

        let output = write(&contract);
        assert_eq!(output.trim(), "count: integer");
    }

    #[test]
    fn writes_empty_object() {
        let contract = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([(
                "config".into(),
                Contract::Object {
                    required: vec![],
                    properties: BTreeMap::new(),
                },
            )]),
        };

        let output = write(&contract);
        assert_eq!(output.trim(), "config {}");
    }

    #[test]
    fn writes_any_as_scalar() {
        let contract = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([("data".into(), Contract::Any)]),
        };

        let output = write(&contract);
        assert_eq!(output.trim(), "data: scalar");
    }

    #[test]
    fn writes_deeply_nested() {
        let contract = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([(
                "level1".into(),
                Contract::Object {
                    required: vec![],
                    properties: BTreeMap::from([(
                        "level2".into(),
                        Contract::Object {
                            required: vec![],
                            properties: BTreeMap::from([(
                                "value".into(),
                                Contract::Scalar {
                                    scalar_type: ScalarType::String,
                                    modifier: ContractModifier::None,
                                },
                            )]),
                        },
                    )]),
                },
            )]),
        };

        let output = write(&contract);
        assert!(output.contains("level1 {"));
        assert!(output.contains("level2 {"));
        assert!(output.contains("value: string"));
    }

    #[test]
    fn writes_array_of_any() {
        let contract = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([(
                "items".into(),
                Contract::Array {
                    items: Box::new(Contract::Any),
                },
            )]),
        };

        let output = write(&contract);
        assert_eq!(output.trim(), "items: []scalar");
    }

    #[test]
    fn writes_scalar_with_all_modifiers() {
        let contract = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([
                (
                    "optional".into(),
                    Contract::Scalar {
                        scalar_type: ScalarType::String,
                        modifier: ContractModifier::Nullable,
                    },
                ),
                (
                    "required".into(),
                    Contract::Scalar {
                        scalar_type: ScalarType::String,
                        modifier: ContractModifier::Required,
                    },
                ),
                (
                    "standard".into(),
                    Contract::Scalar {
                        scalar_type: ScalarType::String,
                        modifier: ContractModifier::None,
                    },
                ),
            ]),
        };

        let output = write(&contract);
        assert!(output.contains("optional: string?"));
        assert!(output.contains("required: string!"));
        assert!(output.contains("standard: string\n") || output.contains("standard: string\r"));
    }

    #[test]
    fn writes_multiple_fields_sorted() {
        let contract = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([
                (
                    "zebra".into(),
                    Contract::Scalar {
                        scalar_type: ScalarType::String,
                        modifier: ContractModifier::None,
                    },
                ),
                (
                    "apple".into(),
                    Contract::Scalar {
                        scalar_type: ScalarType::String,
                        modifier: ContractModifier::None,
                    },
                ),
                (
                    "mango".into(),
                    Contract::Scalar {
                        scalar_type: ScalarType::String,
                        modifier: ContractModifier::None,
                    },
                ),
            ]),
        };

        let output = write(&contract);
        let apple_pos = output.find("apple").unwrap();
        let mango_pos = output.find("mango").unwrap();
        let zebra_pos = output.find("zebra").unwrap();
        // BTreeMap is sorted, so fields should be in alphabetical order
        assert!(apple_pos < mango_pos);
        assert!(mango_pos < zebra_pos);
    }

    #[test]
    fn roundtrip_complex_contract() {
        let input = r#"page {
  featured {
    excerpt: scalar
    title: scalar
    url: scalar
  }
  heading: scalar
  introduction: scalar
}
posts: []{
  author {
    name: scalar
  }
  categories: []scalar
  excerpt: scalar
  published_at: scalar
  tags: []scalar
  title: scalar
}
profile {
  name: scalar
}
site {
  categories: []{
    count: scalar
    name: scalar
    slug: scalar
  }
  copyright {
    owner: scalar
    year: scalar
  }
  description: scalar
  navigation: []{
    label: scalar
    url: scalar
  }
  title: scalar
}
"#;

        let contract = crate::parse(input).unwrap();
        let output = write(&contract);
        let reparsed = crate::parse(&output).unwrap();

        match (&contract, &reparsed) {
            (
                Contract::Object {
                    properties: props1, ..
                },
                Contract::Object {
                    properties: props2, ..
                },
            ) => {
                assert_eq!(props1.len(), props2.len());
                // Verify specific fields
                assert!(props1.contains_key("page"));
                assert!(props1.contains_key("posts"));
                assert!(props1.contains_key("profile"));
                assert!(props1.contains_key("site"));
            }
            _ => panic!("expected objects"),
        }
    }

    #[test]
    fn writes_added_field() {
        let file = ContractFileWithDiff {
            types: BTreeMap::new(),
            fields: BTreeMap::from([(
                "email".into(),
                ContractField::added(Contract::Scalar {
                    scalar_type: ScalarType::String,
                    modifier: ContractModifier::None,
                }),
            )]),
        };

        let output = write_with_diff(&file);
        assert_eq!(output.trim(), "+ email: string");
    }

    #[test]
    fn writes_removed_field() {
        let file = ContractFileWithDiff {
            types: BTreeMap::new(),
            fields: BTreeMap::from([(
                "legacyId".into(),
                ContractField::removed(Contract::Scalar {
                    scalar_type: ScalarType::Integer,
                    modifier: ContractModifier::None,
                }),
            )]),
        };

        let output = write_with_diff(&file);
        assert_eq!(output.trim(), "- legacyId: integer");
    }

    #[test]
    fn writes_changed_field() {
        let file = ContractFileWithDiff {
            types: BTreeMap::new(),
            fields: BTreeMap::from([(
                "age".into(),
                ContractField::changed(
                    Contract::Scalar {
                        scalar_type: ScalarType::Integer,
                        modifier: ContractModifier::None,
                    },
                    Contract::Scalar {
                        scalar_type: ScalarType::Scalar,
                        modifier: ContractModifier::None,
                    },
                ),
            )]),
        };

        let output = write_with_diff(&file);
        assert_eq!(output.trim(), "* age: integer -> scalar");
    }

    #[test]
    fn writes_modifier_change() {
        let file = ContractFileWithDiff {
            types: BTreeMap::new(),
            fields: BTreeMap::from([(
                "bio".into(),
                ContractField::changed(
                    Contract::Scalar {
                        scalar_type: ScalarType::String,
                        modifier: ContractModifier::None,
                    },
                    Contract::Scalar {
                        scalar_type: ScalarType::String,
                        modifier: ContractModifier::Nullable,
                    },
                ),
            )]),
        };

        let output = write_with_diff(&file);
        assert_eq!(output.trim(), "* bio: string -> string?");
    }

    #[test]
    fn writes_mixed_diff_markers() {
        let file = ContractFileWithDiff {
            types: BTreeMap::new(),
            fields: BTreeMap::from([
                (
                    "age".into(),
                    ContractField::changed(
                        Contract::Scalar {
                            scalar_type: ScalarType::Integer,
                            modifier: ContractModifier::None,
                        },
                        Contract::Scalar {
                            scalar_type: ScalarType::Scalar,
                            modifier: ContractModifier::None,
                        },
                    ),
                ),
                (
                    "email".into(),
                    ContractField::added(Contract::Scalar {
                        scalar_type: ScalarType::String,
                        modifier: ContractModifier::None,
                    }),
                ),
                (
                    "legacyId".into(),
                    ContractField::removed(Contract::Scalar {
                        scalar_type: ScalarType::Integer,
                        modifier: ContractModifier::None,
                    }),
                ),
                (
                    "name".into(),
                    ContractField::new(Contract::Scalar {
                        scalar_type: ScalarType::String,
                        modifier: ContractModifier::None,
                    }),
                ),
            ]),
        };

        let output = write_with_diff(&file);
        assert!(output.contains("* age: integer -> scalar"));
        assert!(output.contains("+ email: string"));
        assert!(output.contains("- legacyId: integer"));
        assert!(output.contains("name: string"));
    }

    #[test]
    fn writes_added_type() {
        let file = ContractFileWithDiff {
            types: BTreeMap::from([(
                "NewType".into(),
                TypeDef {
                    marker: Some(DiffMarker::Added),
                    contract: Contract::Object {
                        required: vec![],
                        properties: BTreeMap::from([(
                            "name".into(),
                            Contract::Scalar {
                                scalar_type: ScalarType::String,
                                modifier: ContractModifier::None,
                            },
                        )]),
                    },
                },
            )]),
            fields: BTreeMap::new(),
        };

        let output = write_with_diff(&file);
        assert!(output.contains("+ type NewType {"));
        assert!(output.contains("name: string"));
    }

    #[test]
    fn writes_removed_type() {
        let file = ContractFileWithDiff {
            types: BTreeMap::from([(
                "OldType".into(),
                TypeDef {
                    marker: Some(DiffMarker::Removed),
                    contract: Contract::Object {
                        required: vec![],
                        properties: BTreeMap::from([(
                            "old".into(),
                            Contract::Scalar {
                                scalar_type: ScalarType::String,
                                modifier: ContractModifier::None,
                            },
                        )]),
                    },
                },
            )]),
            fields: BTreeMap::new(),
        };

        let output = write_with_diff(&file);
        assert!(output.contains("- type OldType {"));
        assert!(output.contains("old: string"));
    }

    #[test]
    fn writes_type_and_fields_with_blank_line() {
        let file = ContractFileWithDiff {
            types: BTreeMap::from([(
                "User".into(),
                TypeDef::new(Contract::Object {
                    required: vec![],
                    properties: BTreeMap::from([(
                        "name".into(),
                        Contract::Scalar {
                            scalar_type: ScalarType::String,
                            modifier: ContractModifier::None,
                        },
                    )]),
                }),
            )]),
            fields: BTreeMap::from([(
                "title".into(),
                ContractField::new(Contract::Scalar {
                    scalar_type: ScalarType::String,
                    modifier: ContractModifier::Required,
                }),
            )]),
        };

        let output = write_with_diff(&file);
        // Should have type definition followed by blank line then fields
        assert!(output.contains("type User {"));
        assert!(output.contains("}\n\n"));
        assert!(output.contains("title: string!"));
    }

    #[test]
    fn roundtrip_diff_contract() {
        let input = r#"
+ type NewType {
    name: string
}

- type OldType {
    old: string
}

type User {
    name: string
}

name: string
+ email: string
- legacyId: integer
* age: integer -> scalar
"#;

        let parsed = crate::parse_file_with_diff(input).unwrap();
        let output = write_with_diff(&parsed);
        let reparsed = crate::parse_file_with_diff(&output).unwrap();

        // Verify type definitions
        assert_eq!(parsed.types.len(), reparsed.types.len());

        // Verify fields
        assert_eq!(parsed.fields.len(), reparsed.fields.len());

        // Check specific markers
        assert!(reparsed.fields.get("email").unwrap().marker == Some(DiffMarker::Added));
        assert!(reparsed.fields.get("legacyId").unwrap().marker == Some(DiffMarker::Removed));
        assert!(reparsed.fields.get("age").unwrap().marker == Some(DiffMarker::Changed));
        assert!(reparsed.fields.get("name").unwrap().marker.is_none());
    }
}
