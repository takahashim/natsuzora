//! Contract diff detection.
//!
//! This module compares two contracts and generates a merged contract
//! with diff markers indicating changes between versions.

use std::collections::BTreeMap;

use crate::parser::ContractFileWithDiff;
use crate::types::{Contract, ContractField, ContractModifier, DiffMarker, ScalarType, TypeDef};

/// Compare two contracts and generate a merged contract with diff markers.
///
/// The `old` contract represents the current (existing) version.
/// The `new` contract represents the next (target) version.
///
/// Returns a `ContractFileWithDiff` with:
/// - Unchanged fields (no marker)
/// - Added fields (`+` marker) - present in new, absent in old
/// - Removed fields (`-` marker) - present in old, absent in new
/// - Changed fields (`*` marker) - different types between old and new
pub fn diff_contracts(old: &Contract, new: &Contract) -> ContractFileWithDiff {
    let mut fields = BTreeMap::new();

    // Extract properties from both contracts
    let old_props = extract_properties(old);
    let new_props = extract_properties(new);

    // Find all unique field names
    let mut all_fields: Vec<&String> = old_props.keys().chain(new_props.keys()).collect();
    all_fields.sort();
    all_fields.dedup();

    for name in all_fields {
        let old_contract = old_props.get(name);
        let new_contract = new_props.get(name);

        let field = match (old_contract, new_contract) {
            // Field exists in both
            (Some(old_c), Some(new_c)) => {
                if contracts_equal(old_c, new_c) {
                    // Unchanged
                    ContractField::new((*old_c).clone())
                } else {
                    // Changed
                    ContractField::changed((*old_c).clone(), (*new_c).clone())
                }
            }
            // Field only in new (added)
            (None, Some(new_c)) => ContractField::added((*new_c).clone()),
            // Field only in old (removed)
            (Some(old_c), None) => ContractField::removed((*old_c).clone()),
            // Neither (shouldn't happen)
            (None, None) => continue,
        };

        fields.insert(name.clone(), field);
    }

    ContractFileWithDiff {
        types: BTreeMap::new(),
        fields,
    }
}

/// Compare two ContractFiles and generate a merged file with diff markers.
///
/// This version handles both type definitions and root fields.
pub fn diff_contract_files(
    old: &crate::parser::ContractFile,
    new: &crate::parser::ContractFile,
) -> ContractFileWithDiff {
    let mut types = BTreeMap::new();
    let mut fields = BTreeMap::new();

    // Diff type definitions
    let mut all_types: Vec<&String> = old.types.keys().chain(new.types.keys()).collect();
    all_types.sort();
    all_types.dedup();

    for name in all_types {
        let old_type = old.types.get(name);
        let new_type = new.types.get(name);

        let type_def = match (old_type, new_type) {
            // Type exists in both
            (Some(old_c), Some(new_c)) => {
                if contracts_equal(old_c, new_c) {
                    // Unchanged
                    TypeDef::new(old_c.clone())
                } else {
                    // For types, we can't use Changed marker
                    // Keep the old type and mark internal fields as changed if needed
                    // For simplicity, we just keep the new version without marker
                    TypeDef::new(new_c.clone())
                }
            }
            // Type only in new (added)
            (None, Some(new_c)) => TypeDef {
                marker: Some(DiffMarker::Added),
                contract: new_c.clone(),
            },
            // Type only in old (removed)
            (Some(old_c), None) => TypeDef {
                marker: Some(DiffMarker::Removed),
                contract: old_c.clone(),
            },
            // Neither (shouldn't happen)
            (None, None) => continue,
        };

        types.insert(name.clone(), type_def);
    }

    // Diff root fields
    let old_props = extract_properties(&old.root);
    let new_props = extract_properties(&new.root);

    let mut all_fields: Vec<&String> = old_props.keys().chain(new_props.keys()).collect();
    all_fields.sort();
    all_fields.dedup();

    for name in all_fields {
        let old_contract = old_props.get(name).copied();
        let new_contract = new_props.get(name).copied();

        let field = match (old_contract, new_contract) {
            // Field exists in both
            (Some(old_c), Some(new_c)) => {
                if contracts_equal(old_c, new_c) {
                    // Unchanged
                    ContractField::new(old_c.clone())
                } else {
                    // Changed
                    ContractField::changed(old_c.clone(), new_c.clone())
                }
            }
            // Field only in new (added)
            (None, Some(new_c)) => ContractField::added(new_c.clone()),
            // Field only in old (removed)
            (Some(old_c), None) => ContractField::removed(old_c.clone()),
            // Neither (shouldn't happen)
            (None, None) => continue,
        };

        fields.insert(name.clone(), field);
    }

    ContractFileWithDiff { types, fields }
}

/// Extract properties from a contract (assumes root is an object).
fn extract_properties(contract: &Contract) -> BTreeMap<String, &Contract> {
    match contract {
        Contract::Object { properties, .. } => {
            properties.iter().map(|(k, v)| (k.clone(), v)).collect()
        }
        _ => BTreeMap::new(),
    }
}

/// Check if two contracts are structurally equal.
fn contracts_equal(a: &Contract, b: &Contract) -> bool {
    match (a, b) {
        (Contract::Any, Contract::Any) => true,
        (
            Contract::Scalar {
                scalar_type: st1,
                modifier: m1,
            },
            Contract::Scalar {
                scalar_type: st2,
                modifier: m2,
            },
        ) => st1 == st2 && m1 == m2,
        (Contract::TypeRef { name: n1 }, Contract::TypeRef { name: n2 }) => n1 == n2,
        (Contract::Array { items: i1 }, Contract::Array { items: i2 }) => {
            contracts_equal(i1, i2)
        }
        (
            Contract::Object {
                required: r1,
                properties: p1,
            },
            Contract::Object {
                required: r2,
                properties: p2,
            },
        ) => {
            if r1 != r2 || p1.len() != p2.len() {
                return false;
            }
            for (key, v1) in p1 {
                match p2.get(key) {
                    Some(v2) if contracts_equal(v1, v2) => continue,
                    _ => return false,
                }
            }
            true
        }
        _ => false,
    }
}

/// Check if the diff has any changes.
pub fn has_changes(diff: &ContractFileWithDiff) -> bool {
    // Check types for markers
    for type_def in diff.types.values() {
        if type_def.marker.is_some() {
            return true;
        }
    }

    // Check fields for markers
    for field in diff.fields.values() {
        if field.marker.is_some() {
            return true;
        }
    }

    false
}

/// Format differences as human-readable output.
pub fn format_diff(diff: &ContractFileWithDiff) -> String {
    let mut output = String::new();

    // Format type changes
    for (name, type_def) in &diff.types {
        if let Some(marker) = &type_def.marker {
            let marker_str = match marker {
                DiffMarker::Added => "+",
                DiffMarker::Removed => "-",
                DiffMarker::Changed => "*",
            };
            output.push_str(&format!("{marker_str} type {name}\n"));
        }
    }

    // Format field changes
    for (name, field) in &diff.fields {
        if let Some(marker) = &field.marker {
            let desc = match marker {
                DiffMarker::Added => {
                    format!("+ {}: (added) {}", name, format_type(&field.current_type))
                }
                DiffMarker::Removed => {
                    format!("- {name}: (removed)")
                }
                DiffMarker::Changed => {
                    if let Some(next) = &field.next_type {
                        format!(
                            "* {}: {} -> {}",
                            name,
                            format_type(&field.current_type),
                            format_type(next)
                        )
                    } else {
                        format!("* {name}: (changed)")
                    }
                }
            };
            output.push_str(&desc);
            output.push('\n');
        }
    }

    output
}

/// Format a contract type as a string.
fn format_type(contract: &Contract) -> String {
    match contract {
        Contract::Any => "any".to_string(),
        Contract::Scalar {
            scalar_type,
            modifier,
        } => {
            let type_str = match scalar_type {
                ScalarType::String => "string",
                ScalarType::Integer => "integer",
                ScalarType::Bool => "bool",
                ScalarType::Scalar => "scalar",
            };
            let modifier_str = match modifier {
                ContractModifier::None => "",
                ContractModifier::Nullable => "?",
                ContractModifier::Required => "!",
            };
            format!("{type_str}{modifier_str}")
        }
        Contract::TypeRef { name } => name.clone(),
        Contract::Array { items } => format!("[]{}", format_type(items)),
        Contract::Object { .. } => "{}".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_string_field() -> Contract {
        Contract::Scalar {
            scalar_type: ScalarType::String,
            modifier: ContractModifier::None,
        }
    }

    fn make_integer_field() -> Contract {
        Contract::Scalar {
            scalar_type: ScalarType::Integer,
            modifier: ContractModifier::None,
        }
    }

    fn make_scalar_field() -> Contract {
        Contract::Scalar {
            scalar_type: ScalarType::Scalar,
            modifier: ContractModifier::None,
        }
    }

    #[test]
    fn detects_no_changes() {
        let contract = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([("name".into(), make_string_field())]),
        };

        let diff = diff_contracts(&contract, &contract);

        assert!(!has_changes(&diff));
        assert_eq!(diff.fields.len(), 1);
        assert!(diff.fields.get("name").unwrap().marker.is_none());
    }

    #[test]
    fn detects_added_field() {
        let old = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([("name".into(), make_string_field())]),
        };

        let new = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([
                ("name".into(), make_string_field()),
                ("email".into(), make_string_field()),
            ]),
        };

        let diff = diff_contracts(&old, &new);

        assert!(has_changes(&diff));
        assert_eq!(diff.fields.len(), 2);
        assert!(diff.fields.get("name").unwrap().marker.is_none());
        assert_eq!(
            diff.fields.get("email").unwrap().marker,
            Some(DiffMarker::Added)
        );
    }

    #[test]
    fn detects_removed_field() {
        let old = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([
                ("name".into(), make_string_field()),
                ("legacy".into(), make_integer_field()),
            ]),
        };

        let new = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([("name".into(), make_string_field())]),
        };

        let diff = diff_contracts(&old, &new);

        assert!(has_changes(&diff));
        assert_eq!(diff.fields.len(), 2);
        assert!(diff.fields.get("name").unwrap().marker.is_none());
        assert_eq!(
            diff.fields.get("legacy").unwrap().marker,
            Some(DiffMarker::Removed)
        );
    }

    #[test]
    fn detects_changed_field() {
        let old = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([("age".into(), make_integer_field())]),
        };

        let new = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([("age".into(), make_scalar_field())]),
        };

        let diff = diff_contracts(&old, &new);

        assert!(has_changes(&diff));
        let age_field = diff.fields.get("age").unwrap();
        assert_eq!(age_field.marker, Some(DiffMarker::Changed));
        assert!(age_field.next_type.is_some());
    }

    #[test]
    fn detects_modifier_change() {
        let old = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([(
                "bio".into(),
                Contract::Scalar {
                    scalar_type: ScalarType::String,
                    modifier: ContractModifier::None,
                },
            )]),
        };

        let new = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([(
                "bio".into(),
                Contract::Scalar {
                    scalar_type: ScalarType::String,
                    modifier: ContractModifier::Nullable,
                },
            )]),
        };

        let diff = diff_contracts(&old, &new);

        assert!(has_changes(&diff));
        let bio_field = diff.fields.get("bio").unwrap();
        assert_eq!(bio_field.marker, Some(DiffMarker::Changed));
    }

    #[test]
    fn formats_diff_output() {
        let old = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([
                ("name".into(), make_string_field()),
                ("age".into(), make_integer_field()),
                ("legacy".into(), make_string_field()),
            ]),
        };

        let new = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([
                ("name".into(), make_string_field()),
                ("age".into(), make_scalar_field()),
                ("email".into(), make_string_field()),
            ]),
        };

        let diff = diff_contracts(&old, &new);
        let output = format_diff(&diff);

        assert!(output.contains("+ email"));
        assert!(output.contains("- legacy"));
        assert!(output.contains("* age: integer -> scalar"));
    }

    #[test]
    fn handles_nested_objects() {
        let old = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([(
                "user".into(),
                Contract::Object {
                    required: vec![],
                    properties: BTreeMap::from([("name".into(), make_string_field())]),
                },
            )]),
        };

        let new = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([(
                "user".into(),
                Contract::Object {
                    required: vec![],
                    properties: BTreeMap::from([
                        ("name".into(), make_string_field()),
                        ("email".into(), make_string_field()),
                    ]),
                },
            )]),
        };

        let diff = diff_contracts(&old, &new);

        // The whole user object is considered changed
        assert!(has_changes(&diff));
        let user_field = diff.fields.get("user").unwrap();
        assert_eq!(user_field.marker, Some(DiffMarker::Changed));
    }

    #[test]
    fn handles_arrays() {
        let old = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([(
                "tags".into(),
                Contract::Array {
                    items: Box::new(make_string_field()),
                },
            )]),
        };

        let new = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([(
                "tags".into(),
                Contract::Array {
                    items: Box::new(make_scalar_field()),
                },
            )]),
        };

        let diff = diff_contracts(&old, &new);

        assert!(has_changes(&diff));
        let tags_field = diff.fields.get("tags").unwrap();
        assert_eq!(tags_field.marker, Some(DiffMarker::Changed));
    }

    #[test]
    fn handles_type_refs() {
        let old = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([(
                "author".into(),
                Contract::TypeRef {
                    name: "Author".into(),
                },
            )]),
        };

        let new = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([(
                "author".into(),
                Contract::TypeRef {
                    name: "Author".into(),
                },
            )]),
        };

        let diff = diff_contracts(&old, &new);

        assert!(!has_changes(&diff));
    }
}
