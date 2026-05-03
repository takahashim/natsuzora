//! Apply diff markers to transition to next generation.
//!
//! This module applies diff markers to a contract file, producing a
//! new contract file representing the "next" generation.

use std::collections::BTreeMap;

use crate::parser::{ContractFile, ContractFileWithDiff};
use crate::types::{Contract, DiffMarker};

/// Apply diff markers to transition from current to next generation.
///
/// This function produces a `ContractFile` representing the next generation:
/// - `+` (Added) fields are included
/// - `-` (Removed) fields are excluded
/// - `*` (Changed) fields use the `next_type`
/// - Fields without markers are kept as-is
pub fn apply_diff(file: &ContractFileWithDiff) -> ContractFile {
    let mut types = BTreeMap::new();
    let mut root_properties = BTreeMap::new();

    // Apply to type definitions
    for (name, type_def) in &file.types {
        match type_def.marker {
            // Added: include in next
            Some(DiffMarker::Added) => {
                types.insert(name.clone(), type_def.contract.clone());
            }
            // Removed: exclude from next
            Some(DiffMarker::Removed) => {
                // Skip this type
            }
            // Changed is not allowed for types, treat as no change
            Some(DiffMarker::Changed) => {
                types.insert(name.clone(), type_def.contract.clone());
            }
            // No marker: include as-is
            None => {
                types.insert(name.clone(), type_def.contract.clone());
            }
        }
    }

    // Apply to root fields
    for (name, field) in &file.fields {
        match field.marker {
            // Added: include in next
            Some(DiffMarker::Added) => {
                root_properties.insert(name.clone(), field.current_type.clone());
            }
            // Removed: exclude from next
            Some(DiffMarker::Removed) => {
                // Skip this field
            }
            // Changed: use next_type
            Some(DiffMarker::Changed) => {
                if let Some(next_type) = &field.next_type {
                    root_properties.insert(name.clone(), next_type.clone());
                } else {
                    // Fallback to current if next is missing
                    root_properties.insert(name.clone(), field.current_type.clone());
                }
            }
            // No marker: include as-is
            None => {
                root_properties.insert(name.clone(), field.current_type.clone());
            }
        }
    }

    ContractFile {
        types,
        root: Contract::Object {
            required: root_properties.keys().cloned().collect(),
            properties: root_properties,
        },
    }
}

/// Apply diff markers to stay at current generation.
///
/// This function produces a `ContractFile` representing the current generation:
/// - `+` (Added) fields are excluded
/// - `-` (Removed) fields are included
/// - `*` (Changed) fields use the `current_type`
/// - Fields without markers are kept as-is
pub fn apply_current(file: &ContractFileWithDiff) -> ContractFile {
    let mut types = BTreeMap::new();
    let mut root_properties = BTreeMap::new();

    // Apply to type definitions
    for (name, type_def) in &file.types {
        match type_def.marker {
            // Added: exclude from current
            Some(DiffMarker::Added) => {
                // Skip this type
            }
            // Removed: include in current
            Some(DiffMarker::Removed) => {
                types.insert(name.clone(), type_def.contract.clone());
            }
            // Changed is not allowed for types, treat as no change
            Some(DiffMarker::Changed) => {
                types.insert(name.clone(), type_def.contract.clone());
            }
            // No marker: include as-is
            None => {
                types.insert(name.clone(), type_def.contract.clone());
            }
        }
    }

    // Apply to root fields
    for (name, field) in &file.fields {
        match field.marker {
            // Added: exclude from current
            Some(DiffMarker::Added) => {
                // Skip this field
            }
            // Removed: include in current
            Some(DiffMarker::Removed) => {
                root_properties.insert(name.clone(), field.current_type.clone());
            }
            // Changed: use current_type
            Some(DiffMarker::Changed) => {
                root_properties.insert(name.clone(), field.current_type.clone());
            }
            // No marker: include as-is
            None => {
                root_properties.insert(name.clone(), field.current_type.clone());
            }
        }
    }

    ContractFile {
        types,
        root: Contract::Object {
            required: root_properties.keys().cloned().collect(),
            properties: root_properties,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ContractField, ContractModifier, ScalarType, TypeDef};

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
    fn apply_diff_keeps_unchanged() {
        let file = ContractFileWithDiff {
            types: BTreeMap::new(),
            fields: BTreeMap::from([(
                "name".into(),
                ContractField::new(make_string_field()),
            )]),
        };

        let result = apply_diff(&file);

        match &result.root {
            Contract::Object { properties, .. } => {
                assert!(properties.contains_key("name"));
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn apply_diff_includes_added() {
        let file = ContractFileWithDiff {
            types: BTreeMap::new(),
            fields: BTreeMap::from([
                ("name".into(), ContractField::new(make_string_field())),
                ("email".into(), ContractField::added(make_string_field())),
            ]),
        };

        let result = apply_diff(&file);

        match &result.root {
            Contract::Object { properties, .. } => {
                assert!(properties.contains_key("name"));
                assert!(properties.contains_key("email"));
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn apply_diff_excludes_removed() {
        let file = ContractFileWithDiff {
            types: BTreeMap::new(),
            fields: BTreeMap::from([
                ("name".into(), ContractField::new(make_string_field())),
                ("legacy".into(), ContractField::removed(make_integer_field())),
            ]),
        };

        let result = apply_diff(&file);

        match &result.root {
            Contract::Object { properties, .. } => {
                assert!(properties.contains_key("name"));
                assert!(!properties.contains_key("legacy"));
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn apply_diff_uses_next_type() {
        let file = ContractFileWithDiff {
            types: BTreeMap::new(),
            fields: BTreeMap::from([(
                "age".into(),
                ContractField::changed(make_integer_field(), make_scalar_field()),
            )]),
        };

        let result = apply_diff(&file);

        match &result.root {
            Contract::Object { properties, .. } => {
                let age = properties.get("age").unwrap();
                match age {
                    Contract::Scalar { scalar_type, .. } => {
                        assert_eq!(*scalar_type, ScalarType::Scalar);
                    }
                    _ => panic!("expected scalar"),
                }
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn apply_current_excludes_added() {
        let file = ContractFileWithDiff {
            types: BTreeMap::new(),
            fields: BTreeMap::from([
                ("name".into(), ContractField::new(make_string_field())),
                ("email".into(), ContractField::added(make_string_field())),
            ]),
        };

        let result = apply_current(&file);

        match &result.root {
            Contract::Object { properties, .. } => {
                assert!(properties.contains_key("name"));
                assert!(!properties.contains_key("email"));
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn apply_current_includes_removed() {
        let file = ContractFileWithDiff {
            types: BTreeMap::new(),
            fields: BTreeMap::from([
                ("name".into(), ContractField::new(make_string_field())),
                ("legacy".into(), ContractField::removed(make_integer_field())),
            ]),
        };

        let result = apply_current(&file);

        match &result.root {
            Contract::Object { properties, .. } => {
                assert!(properties.contains_key("name"));
                assert!(properties.contains_key("legacy"));
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn apply_current_uses_current_type() {
        let file = ContractFileWithDiff {
            types: BTreeMap::new(),
            fields: BTreeMap::from([(
                "age".into(),
                ContractField::changed(make_integer_field(), make_scalar_field()),
            )]),
        };

        let result = apply_current(&file);

        match &result.root {
            Contract::Object { properties, .. } => {
                let age = properties.get("age").unwrap();
                match age {
                    Contract::Scalar { scalar_type, .. } => {
                        assert_eq!(*scalar_type, ScalarType::Integer);
                    }
                    _ => panic!("expected scalar"),
                }
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn apply_diff_includes_added_type() {
        let file = ContractFileWithDiff {
            types: BTreeMap::from([(
                "NewType".into(),
                TypeDef {
                    marker: Some(DiffMarker::Added),
                    contract: Contract::Object {
                        required: vec![],
                        properties: BTreeMap::from([("value".into(), make_string_field())]),
                    },
                },
            )]),
            fields: BTreeMap::new(),
        };

        let result = apply_diff(&file);

        assert!(result.types.contains_key("NewType"));
    }

    #[test]
    fn apply_diff_excludes_removed_type() {
        let file = ContractFileWithDiff {
            types: BTreeMap::from([(
                "OldType".into(),
                TypeDef {
                    marker: Some(DiffMarker::Removed),
                    contract: Contract::Object {
                        required: vec![],
                        properties: BTreeMap::from([("old".into(), make_string_field())]),
                    },
                },
            )]),
            fields: BTreeMap::new(),
        };

        let result = apply_diff(&file);

        assert!(!result.types.contains_key("OldType"));
    }

    #[test]
    fn apply_current_excludes_added_type() {
        let file = ContractFileWithDiff {
            types: BTreeMap::from([(
                "NewType".into(),
                TypeDef {
                    marker: Some(DiffMarker::Added),
                    contract: Contract::Object {
                        required: vec![],
                        properties: BTreeMap::from([("value".into(), make_string_field())]),
                    },
                },
            )]),
            fields: BTreeMap::new(),
        };

        let result = apply_current(&file);

        assert!(!result.types.contains_key("NewType"));
    }

    #[test]
    fn apply_current_includes_removed_type() {
        let file = ContractFileWithDiff {
            types: BTreeMap::from([(
                "OldType".into(),
                TypeDef {
                    marker: Some(DiffMarker::Removed),
                    contract: Contract::Object {
                        required: vec![],
                        properties: BTreeMap::from([("old".into(), make_string_field())]),
                    },
                },
            )]),
            fields: BTreeMap::new(),
        };

        let result = apply_current(&file);

        assert!(result.types.contains_key("OldType"));
    }

    // ---- Edge cases (条件分岐を持つ箇所のみ) ----

    #[test]
    fn apply_diff_changed_marker_on_type_is_treated_as_no_change() {
        // 仕様上 type def に Changed marker は付かないが、parser bug などで来た場合の防御。
        // Changed → 既存 contract をそのまま含める (apply_diff line 34-36)
        let original_contract = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([("value".into(), make_string_field())]),
        };
        let file = ContractFileWithDiff {
            types: BTreeMap::from([(
                "WeirdType".into(),
                TypeDef {
                    marker: Some(DiffMarker::Changed),
                    contract: original_contract.clone(),
                },
            )]),
            fields: BTreeMap::new(),
        };

        let result = apply_diff(&file);

        assert!(result.types.contains_key("WeirdType"));
        // 中身も維持されている (next_type のような概念が type にはないので current のまま)
        let stored = result.types.get("WeirdType").expect("type kept");
        assert!(matches!(stored, Contract::Object { .. }));
    }

    #[test]
    fn apply_diff_changed_field_without_next_type_falls_back_to_current() {
        // ContractField::changed(current, next) は next が常に Some を保証するが、
        // 直接構築すると next_type: None もあり得る (parser bug 等)。
        // この場合 apply_diff は current_type を fallback として使う (apply.rs line 56-62)。
        let field = ContractField {
            marker: Some(DiffMarker::Changed),
            current_type: make_integer_field(),
            next_type: None,
        };
        let file = ContractFileWithDiff {
            types: BTreeMap::new(),
            fields: BTreeMap::from([("count".into(), field)]),
        };

        let result = apply_diff(&file);

        match &result.root {
            Contract::Object { properties, .. } => {
                let count = properties.get("count").expect("count field");
                match count {
                    Contract::Scalar { scalar_type, .. } => {
                        // next_type: None なので current_type (Integer) にフォールバック
                        assert_eq!(*scalar_type, ScalarType::Integer);
                    }
                    _ => panic!("expected scalar"),
                }
            }
            _ => panic!("expected object"),
        }
    }
}
