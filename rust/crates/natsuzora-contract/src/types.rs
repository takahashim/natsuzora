//! Contract type definitions.
//!
//! This module contains the core types for representing data contracts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Diff marker for 2-generation contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffMarker {
    /// `+` - Field will be added in next generation
    Added,
    /// `-` - Field will be removed in next generation
    Removed,
    /// `*` - Field type will change in next generation
    Changed,
}

/// Validation target for 2-generation contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ValidationTarget {
    /// Validate against current generation (default)
    #[default]
    Current,
    /// Validate against next generation
    Next,
}

/// Scalar type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScalarType {
    /// String type only
    String,
    /// Integer type only
    Integer,
    /// Boolean type only (for truthiness checks)
    Bool,
    /// String or Integer (stringifiable values, does NOT include bool)
    Scalar,
}

/// Modifier for null/empty handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContractModifier {
    /// Default: null not allowed, empty string allowed
    #[default]
    None,
    /// `?` modifier: null allowed
    Nullable,
    /// `!` modifier: null not allowed, empty string not allowed
    Required,
}

/// Contract type representing data structure constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Contract {
    /// Any value (unconstrained)
    Any,
    /// Scalar value with type and modifier
    Scalar {
        #[serde(rename = "type")]
        scalar_type: ScalarType,
        #[serde(default, skip_serializing_if = "is_default_modifier")]
        modifier: ContractModifier,
    },
    /// Object with named properties
    Object {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        required: Vec<String>,
        properties: BTreeMap<String, Contract>,
    },
    /// Array of items
    Array { items: Box<Contract> },
    /// Type reference (used during parsing, resolved before use)
    #[serde(rename = "ref")]
    TypeRef { name: String },
}

fn is_default_modifier(m: &ContractModifier) -> bool {
    matches!(m, ContractModifier::None)
}

/// A field with optional diff marker and type change information.
#[derive(Debug, Clone)]
pub struct ContractField {
    /// Diff marker (+, -, *)
    pub marker: Option<DiffMarker>,
    /// Current type (used when marker is None, Removed, or Changed)
    pub current_type: Contract,
    /// Next type (used only when marker is Changed)
    pub next_type: Option<Contract>,
}

impl ContractField {
    /// Create a field without diff marker.
    pub fn new(contract: Contract) -> Self {
        Self {
            marker: None,
            current_type: contract,
            next_type: None,
        }
    }

    /// Create an added field (+).
    pub fn added(contract: Contract) -> Self {
        Self {
            marker: Some(DiffMarker::Added),
            current_type: contract,
            next_type: None,
        }
    }

    /// Create a removed field (-).
    pub fn removed(contract: Contract) -> Self {
        Self {
            marker: Some(DiffMarker::Removed),
            current_type: contract,
            next_type: None,
        }
    }

    /// Create a changed field (*).
    pub fn changed(current: Contract, next: Contract) -> Self {
        Self {
            marker: Some(DiffMarker::Changed),
            current_type: current,
            next_type: Some(next),
        }
    }

    /// Get the contract for the specified target generation.
    pub fn for_target(&self, target: ValidationTarget) -> Option<&Contract> {
        match (self.marker, target) {
            // No marker: same for both
            (None, _) => Some(&self.current_type),
            // Added: only in next
            (Some(DiffMarker::Added), ValidationTarget::Current) => None,
            (Some(DiffMarker::Added), ValidationTarget::Next) => Some(&self.current_type),
            // Removed: only in current
            (Some(DiffMarker::Removed), ValidationTarget::Current) => Some(&self.current_type),
            (Some(DiffMarker::Removed), ValidationTarget::Next) => None,
            // Changed: current_type for current, next_type for next
            (Some(DiffMarker::Changed), ValidationTarget::Current) => Some(&self.current_type),
            (Some(DiffMarker::Changed), ValidationTarget::Next) => self.next_type.as_ref(),
        }
    }
}

/// A type definition with optional diff marker.
#[derive(Debug, Clone)]
pub struct TypeDef {
    /// Diff marker (+ or -)
    pub marker: Option<DiffMarker>,
    /// The type's contract
    pub contract: Contract,
}

impl TypeDef {
    /// Create a type definition without diff marker.
    pub fn new(contract: Contract) -> Self {
        Self {
            marker: None,
            contract,
        }
    }

    /// Check if this type is available for the specified target.
    pub fn is_available(&self, target: ValidationTarget) -> bool {
        match (self.marker, target) {
            (None, _) => true,
            (Some(DiffMarker::Added), ValidationTarget::Current) => false,
            (Some(DiffMarker::Added), ValidationTarget::Next) => true,
            (Some(DiffMarker::Removed), ValidationTarget::Current) => true,
            (Some(DiffMarker::Removed), ValidationTarget::Next) => false,
            (Some(DiffMarker::Changed), _) => false, // Changed not allowed for type defs
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_serializes_to_json() {
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

        let json = serde_json::to_string(&contract).unwrap();
        assert!(json.contains("\"kind\":\"object\""));
        assert!(json.contains("\"name\""));
    }

    #[test]
    fn contract_deserializes_from_json() {
        let json = r#"{"kind":"scalar","type":"string"}"#;
        let contract: Contract = serde_json::from_str(json).unwrap();

        match contract {
            Contract::Scalar {
                scalar_type,
                modifier,
            } => {
                assert_eq!(scalar_type, ScalarType::String);
                assert_eq!(modifier, ContractModifier::None);
            }
            _ => panic!("expected scalar"),
        }
    }

    #[test]
    fn default_modifier_is_none() {
        assert_eq!(ContractModifier::default(), ContractModifier::None);
    }
}
