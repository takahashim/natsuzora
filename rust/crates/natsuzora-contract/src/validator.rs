//! JSON data validation against contracts.
//!
//! This module validates JSON data against a contract specification.

use std::collections::BTreeMap;

use thiserror::Error;

use crate::parser::ContractFileWithDiff;
use crate::types::{Contract, ContractModifier, ScalarType, ValidationTarget};

/// Errors that can occur during validation.
#[derive(Debug, Error)]
#[error("{message} at {path}")]
pub struct ValidationError {
    pub path: String,
    pub message: String,
}

/// Validate JSON data against a contract.
///
/// # Example
///
/// ```rust
/// use natsuzora_contract::{validate, Contract, ScalarType, ContractModifier};
/// use serde_json::json;
/// use std::collections::BTreeMap;
///
/// let contract = Contract::Object {
///     required: vec!["name".into()],
///     properties: BTreeMap::from([(
///         "name".into(),
///         Contract::Scalar {
///             scalar_type: ScalarType::String,
///             modifier: ContractModifier::None,
///         },
///     )]),
/// };
///
/// // Valid data
/// assert!(validate(&contract, &json!({"name": "Alice"})).is_ok());
///
/// // Missing required field
/// assert!(validate(&contract, &json!({})).is_err());
/// ```
pub fn validate(contract: &Contract, data: &serde_json::Value) -> Result<(), ValidationError> {
    let mut path = Vec::new();
    validate_node(contract, data, &mut path)
}

/// Validate JSON data against a contract file with diff markers.
///
/// The `target` parameter specifies which generation to validate against:
/// - `ValidationTarget::Current`: Validate against current generation (default)
/// - `ValidationTarget::Next`: Validate against next generation
///
/// Fields and types are filtered based on their diff markers:
/// - `+` (Added): Only exists in Next
/// - `-` (Removed): Only exists in Current
/// - `*` (Changed): Uses current_type for Current, next_type for Next
/// - No marker: Exists in both generations
pub fn validate_with_target(
    file: &ContractFileWithDiff,
    data: &serde_json::Value,
    target: ValidationTarget,
) -> Result<(), ValidationError> {
    // Build the contract for the specified target
    let contract = build_contract_for_target(file, target)?;
    validate(&contract, data)
}

/// Build a resolved Contract from ContractFileWithDiff for the specified target.
fn build_contract_for_target(
    file: &ContractFileWithDiff,
    target: ValidationTarget,
) -> Result<Contract, ValidationError> {
    // Build type definitions map for the target (excluding unavailable types)
    let mut type_defs: BTreeMap<String, Contract> = BTreeMap::new();
    for (name, type_def) in &file.types {
        if type_def.is_available(target) {
            type_defs.insert(name.clone(), type_def.contract.clone());
        }
    }

    // Build root object properties for the target
    let mut properties: BTreeMap<String, Contract> = BTreeMap::new();
    let mut required: Vec<String> = Vec::new();

    for (name, field) in &file.fields {
        if let Some(contract) = field.for_target(target) {
            // Resolve type references
            let resolved = resolve_type_refs(contract, &type_defs)?;
            properties.insert(name.clone(), resolved);

            // All present fields are required (loose validation ignores extra fields)
            required.push(name.clone());
        }
    }

    Ok(Contract::Object {
        required,
        properties,
    })
}

/// Resolve TypeRef in a contract using type definitions.
fn resolve_type_refs(
    contract: &Contract,
    type_defs: &BTreeMap<String, Contract>,
) -> Result<Contract, ValidationError> {
    match contract {
        Contract::TypeRef { name } => {
            type_defs.get(name).cloned().ok_or_else(|| ValidationError {
                path: String::new(),
                message: format!("undefined type '{name}'"),
            })
        }
        Contract::Array { items } => Ok(Contract::Array {
            items: Box::new(resolve_type_refs(items, type_defs)?),
        }),
        Contract::Object {
            required,
            properties,
        } => {
            let mut resolved_props = BTreeMap::new();
            for (key, value) in properties {
                resolved_props.insert(key.clone(), resolve_type_refs(value, type_defs)?);
            }
            Ok(Contract::Object {
                required: required.clone(),
                properties: resolved_props,
            })
        }
        _ => Ok(contract.clone()),
    }
}

// ============================================================================
// Internal: Path Rendering
// ============================================================================

#[derive(Clone)]
enum PathSegment {
    Key(String),
    Index(usize),
}

fn render_path(path: &[PathSegment]) -> String {
    let mut repr = String::from("$");
    for segment in path {
        match segment {
            PathSegment::Key(key) => {
                repr.push('.');
                repr.push_str(key);
            }
            PathSegment::Index(idx) => {
                repr.push('[');
                repr.push_str(&idx.to_string());
                repr.push(']');
            }
        }
    }
    repr
}

// ============================================================================
// Internal: Validation Logic
// ============================================================================

fn validate_node(
    contract: &Contract,
    data: &serde_json::Value,
    path: &mut Vec<PathSegment>,
) -> Result<(), ValidationError> {
    match contract {
        Contract::Any => Ok(()),
        Contract::Scalar {
            scalar_type,
            modifier,
        } => validate_scalar(*scalar_type, *modifier, data, path),
        Contract::Object {
            required,
            properties,
        } => validate_object(required, properties, data, path),
        Contract::Array { items } => validate_array(items, data, path),
        Contract::TypeRef { name } => Err(ValidationError {
            path: render_path(path),
            message: format!("unresolved type reference '{name}'"),
        }),
    }
}

fn validate_scalar(
    scalar_type: ScalarType,
    modifier: ContractModifier,
    data: &serde_json::Value,
    path: &mut Vec<PathSegment>,
) -> Result<(), ValidationError> {
    // Handle null
    if data.is_null() {
        return if matches!(modifier, ContractModifier::Nullable) {
            Ok(())
        } else {
            Err(ValidationError {
                path: render_path(path),
                message: "null is not allowed".into(),
            })
        };
    }

    // Check type
    let valid = match scalar_type {
        ScalarType::String => {
            if let Some(s) = data.as_str() {
                if matches!(modifier, ContractModifier::Required) && s.is_empty() {
                    return Err(ValidationError {
                        path: render_path(path),
                        message: "empty string is not allowed".into(),
                    });
                }
                true
            } else {
                false
            }
        }
        ScalarType::Integer => data.is_i64() || data.is_u64(),
        ScalarType::Bool => data.is_boolean(),
        ScalarType::Scalar => {
            if let Some(s) = data.as_str() {
                if matches!(modifier, ContractModifier::Required) && s.is_empty() {
                    return Err(ValidationError {
                        path: render_path(path),
                        message: "empty string is not allowed".into(),
                    });
                }
                true
            } else {
                data.is_i64() || data.is_u64()
            }
        }
    };

    if valid {
        Ok(())
    } else {
        Err(ValidationError {
            path: render_path(path),
            message: format!("expected {scalar_type:?}"),
        })
    }
}

fn validate_object(
    required: &[String],
    properties: &BTreeMap<String, Contract>,
    data: &serde_json::Value,
    path: &mut Vec<PathSegment>,
) -> Result<(), ValidationError> {
    let obj = data.as_object().ok_or_else(|| ValidationError {
        path: render_path(path),
        message: "expected object".into(),
    })?;

    for key in required {
        if !obj.contains_key(key) {
            let mut path = path.clone();
            path.push(PathSegment::Key(key.clone()));
            return Err(ValidationError {
                path: render_path(&path),
                message: "missing required property".into(),
            });
        }
    }

    for (key, child_contract) in properties {
        if let Some(value) = obj.get(key) {
            path.push(PathSegment::Key(key.clone()));
            validate_node(child_contract, value, path)?;
            path.pop();
        }
    }
    Ok(())
}

fn validate_array(
    items: &Contract,
    data: &serde_json::Value,
    path: &mut Vec<PathSegment>,
) -> Result<(), ValidationError> {
    let arr = data.as_array().ok_or_else(|| ValidationError {
        path: render_path(path),
        message: "expected array".into(),
    })?;
    for (idx, value) in arr.iter().enumerate() {
        path.push(PathSegment::Index(idx));
        validate_node(items, value, path)?;
        path.pop();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ContractField, TypeDef};
    use serde_json::json;

    #[test]
    fn validates_simple_object() {
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

        assert!(validate(&contract, &json!({"name": "Alice"})).is_ok());
        assert!(validate(&contract, &json!({})).is_err());
    }

    #[test]
    fn validates_arrays_and_objects() {
        let contract = Contract::Object {
            required: vec!["items".into()],
            properties: BTreeMap::from([(
                "items".into(),
                Contract::Array {
                    items: Box::new(Contract::Object {
                        required: vec!["title".into()],
                        properties: BTreeMap::from([(
                            "title".into(),
                            Contract::Scalar {
                                scalar_type: ScalarType::String,
                                modifier: ContractModifier::None,
                            },
                        )]),
                    }),
                },
            )]),
        };

        assert!(validate(
            &contract,
            &json!({"items": [{"title": "Hello"}, {"title": "World"}]})
        )
        .is_ok());

        let err = validate(
            &contract,
            &json!({"items": [{"title": "Ok"}, {"title": 123}]}),
        )
        .unwrap_err();
        assert!(err.message.contains("expected"));
        assert_eq!(err.path, "$.items[1].title");
    }

    #[test]
    fn validates_nullable_modifier() {
        let contract = Contract::Scalar {
            scalar_type: ScalarType::String,
            modifier: ContractModifier::Nullable,
        };

        assert!(validate(&contract, &json!(null)).is_ok());
        assert!(validate(&contract, &json!("hello")).is_ok());
    }

    #[test]
    fn validates_required_modifier() {
        let contract = Contract::Scalar {
            scalar_type: ScalarType::String,
            modifier: ContractModifier::Required,
        };

        assert!(validate(&contract, &json!("hello")).is_ok());
        assert!(validate(&contract, &json!("")).is_err());
        assert!(validate(&contract, &json!(null)).is_err());
    }

    #[test]
    fn validates_scalar_type() {
        let contract = Contract::Scalar {
            scalar_type: ScalarType::Scalar,
            modifier: ContractModifier::None,
        };

        assert!(validate(&contract, &json!("hello")).is_ok());
        assert!(validate(&contract, &json!(42)).is_ok());
        assert!(validate(&contract, &json!(true)).is_err());
        assert!(validate(&contract, &json!(null)).is_err());
    }

    #[test]
    fn error_path_is_correct() {
        let contract = Contract::Object {
            required: vec![],
            properties: BTreeMap::from([(
                "user".into(),
                Contract::Object {
                    required: vec!["email".into()],
                    properties: BTreeMap::from([(
                        "email".into(),
                        Contract::Scalar {
                            scalar_type: ScalarType::String,
                            modifier: ContractModifier::None,
                        },
                    )]),
                },
            )]),
        };

        let err = validate(&contract, &json!({"user": {}})).unwrap_err();
        assert_eq!(err.path, "$.user.email");
    }

    #[test]
    fn validates_added_field_current() {
        let file = ContractFileWithDiff {
            types: BTreeMap::new(),
            fields: BTreeMap::from([
                (
                    "name".into(),
                    ContractField::new(Contract::Scalar {
                        scalar_type: ScalarType::String,
                        modifier: ContractModifier::None,
                    }),
                ),
                (
                    "email".into(),
                    ContractField::added(Contract::Scalar {
                        scalar_type: ScalarType::String,
                        modifier: ContractModifier::None,
                    }),
                ),
            ]),
        };

        // Current: email is not required (added field)
        assert!(validate_with_target(&file, &json!({"name": "Alice"}), ValidationTarget::Current).is_ok());
        // email can be present, but extra fields are ignored
        assert!(validate_with_target(&file, &json!({"name": "Alice", "email": "test@example.com"}), ValidationTarget::Current).is_ok());
    }

    #[test]
    fn validates_added_field_next() {
        let file = ContractFileWithDiff {
            types: BTreeMap::new(),
            fields: BTreeMap::from([
                (
                    "name".into(),
                    ContractField::new(Contract::Scalar {
                        scalar_type: ScalarType::String,
                        modifier: ContractModifier::None,
                    }),
                ),
                (
                    "email".into(),
                    ContractField::added(Contract::Scalar {
                        scalar_type: ScalarType::String,
                        modifier: ContractModifier::None,
                    }),
                ),
            ]),
        };

        // Next: email is required
        assert!(validate_with_target(&file, &json!({"name": "Alice"}), ValidationTarget::Next).is_err());
        assert!(validate_with_target(&file, &json!({"name": "Alice", "email": "test@example.com"}), ValidationTarget::Next).is_ok());
    }

    #[test]
    fn validates_removed_field_current() {
        let file = ContractFileWithDiff {
            types: BTreeMap::new(),
            fields: BTreeMap::from([
                (
                    "name".into(),
                    ContractField::new(Contract::Scalar {
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
            ]),
        };

        // Current: legacyId is required
        assert!(validate_with_target(&file, &json!({"name": "Alice"}), ValidationTarget::Current).is_err());
        assert!(validate_with_target(&file, &json!({"name": "Alice", "legacyId": 123}), ValidationTarget::Current).is_ok());
    }

    #[test]
    fn validates_removed_field_next() {
        let file = ContractFileWithDiff {
            types: BTreeMap::new(),
            fields: BTreeMap::from([
                (
                    "name".into(),
                    ContractField::new(Contract::Scalar {
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
            ]),
        };

        // Next: legacyId is not required (removed field)
        assert!(validate_with_target(&file, &json!({"name": "Alice"}), ValidationTarget::Next).is_ok());
    }

    #[test]
    fn validates_changed_field_current() {
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

        // Current: age must be integer
        assert!(validate_with_target(&file, &json!({"age": 30}), ValidationTarget::Current).is_ok());
        assert!(validate_with_target(&file, &json!({"age": "30"}), ValidationTarget::Current).is_err());
    }

    #[test]
    fn validates_changed_field_next() {
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

        // Next: age can be integer or string (scalar)
        assert!(validate_with_target(&file, &json!({"age": 30}), ValidationTarget::Next).is_ok());
        assert!(validate_with_target(&file, &json!({"age": "30"}), ValidationTarget::Next).is_ok());
    }

    #[test]
    fn validates_with_type_definitions() {
        let file = ContractFileWithDiff {
            types: BTreeMap::from([(
                "User".into(),
                TypeDef::new(Contract::Object {
                    required: vec!["name".into()],
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
                "user".into(),
                ContractField::new(Contract::TypeRef { name: "User".into() }),
            )]),
        };

        assert!(validate_with_target(&file, &json!({"user": {"name": "Alice"}}), ValidationTarget::Current).is_ok());
        assert!(validate_with_target(&file, &json!({"user": {}}), ValidationTarget::Current).is_err());
    }

    #[test]
    fn validates_added_type_current() {
        use crate::types::DiffMarker;

        let file = ContractFileWithDiff {
            types: BTreeMap::from([(
                "NewType".into(),
                TypeDef {
                    marker: Some(DiffMarker::Added),
                    contract: Contract::Object {
                        required: vec!["value".into()],
                        properties: BTreeMap::from([(
                            "value".into(),
                            Contract::Scalar {
                                scalar_type: ScalarType::String,
                                modifier: ContractModifier::None,
                            },
                        )]),
                    },
                },
            )]),
            fields: BTreeMap::from([(
                "data".into(),
                ContractField::added(Contract::TypeRef { name: "NewType".into() }),
            )]),
        };

        // Current: data field with NewType is not required (both are added)
        assert!(validate_with_target(&file, &json!({}), ValidationTarget::Current).is_ok());
    }

    #[test]
    fn validates_added_type_next() {
        use crate::types::DiffMarker;

        let file = ContractFileWithDiff {
            types: BTreeMap::from([(
                "NewType".into(),
                TypeDef {
                    marker: Some(DiffMarker::Added),
                    contract: Contract::Object {
                        required: vec!["value".into()],
                        properties: BTreeMap::from([(
                            "value".into(),
                            Contract::Scalar {
                                scalar_type: ScalarType::String,
                                modifier: ContractModifier::None,
                            },
                        )]),
                    },
                },
            )]),
            fields: BTreeMap::from([(
                "data".into(),
                ContractField::added(Contract::TypeRef { name: "NewType".into() }),
            )]),
        };

        // Next: data field with NewType is required
        assert!(validate_with_target(&file, &json!({}), ValidationTarget::Next).is_err());
        assert!(validate_with_target(&file, &json!({"data": {"value": "test"}}), ValidationTarget::Next).is_ok());
    }
}
