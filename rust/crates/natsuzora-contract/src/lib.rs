//! Subaru - Schema language for data structure validation.
//!
//! Subaru describes the data structure that JSON data must conform to.
//!
//! This crate provides:
//! - **Types**: Core schema types ([`Contract`], [`ScalarType`], [`ContractModifier`])
//! - **Extraction**: Extract schemas from Natsuzora templates ([`extract_contract`])
//! - **Validation**: Validate JSON data against schemas ([`validate`])
//! - **Parsing/Writing**: Schema notation parser and writer ([`parse`], [`write`])
//! - **Template Checking**: Check templates against schemas ([`check_template`])
//!
//! # Example
//!
//! ```rust,ignore
//! use natsuzora_contract::{extract_contract, validate, check_template};
//!
//! // Extract a schema from a template
//! let contract = extract_contract(&template, &mut loader)?;
//!
//! // Validate JSON data against the schema
//! validate(&contract, &json_data)?;
//!
//! // Check a template against a schema
//! let errors = check_template(&template, &contract, &mut loader);
//! ```

// Re-export IncludeLoader from natsuzora::ast for convenience
pub use natsuzora::ast::{IncludeLoader, LoaderError};

mod apply;
mod diff;
mod extractor;
mod parser;
mod template_checker;
mod types;
mod validator;
mod writer;

// Re-export types
pub use types::{Contract, ContractField, ContractModifier, DiffMarker, ScalarType, TypeDef, ValidationTarget};

// Re-export extractor
pub use extractor::{extract_contract, ContractError};

// Re-export validator
pub use validator::{validate, validate_with_target, ValidationError};

// Re-export parser
pub use parser::{parse, parse_file, parse_file_with_diff, ContractFile, ContractFileWithDiff, ParseError};

// Re-export writer
pub use writer::{write, write_with_diff};

// Re-export template checker
pub use template_checker::{check_template, TemplateCheckError};

// Re-export diff
pub use diff::{diff_contract_files, diff_contracts, format_diff, has_changes};

// Re-export apply
pub use apply::{apply_current, apply_diff};

#[cfg(test)]
mod tests {
    use super::*;
    use natsuzora::ast::Template;
    use serde_json::json;
    use std::collections::BTreeMap;

    struct TestLoader;

    impl IncludeLoader for TestLoader {
        fn load(&mut self, _: &str) -> Result<Template, LoaderError> {
            unreachable!("no include expected")
        }
    }

    #[test]
    fn integration_extract_and_validate() {
        let template = natsuzora::ast::parse("Hello {[ user.name ]}!").unwrap();
        let mut loader = TestLoader;
        let contract = extract_contract(&template, &mut loader).unwrap();

        // Valid data
        assert!(validate(&contract, &json!({"user": {"name": "Alice"}})).is_ok());

        // Missing field
        assert!(validate(&contract, &json!({})).is_err());
    }

    #[test]
    fn integration_parse_and_validate() {
        let contract_source = r#"
            user {
                name: string!
                age: integer
            }
        "#;

        let contract = parse(contract_source).unwrap();

        // Valid data
        assert!(validate(&contract, &json!({"user": {"name": "Alice", "age": 30}})).is_ok());

        // Empty string not allowed with !
        let result = validate(&contract, &json!({"user": {"name": "", "age": 30}}));
        assert!(result.is_err());
    }

    #[test]
    fn contract_roundtrip() {
        let contract = Contract::Object {
            required: vec!["name".into()],
            properties: BTreeMap::from([
                (
                    "name".into(),
                    Contract::Scalar {
                        scalar_type: ScalarType::String,
                        modifier: ContractModifier::Required,
                    },
                ),
                (
                    "tags".into(),
                    Contract::Array {
                        items: Box::new(Contract::Scalar {
                            scalar_type: ScalarType::String,
                            modifier: ContractModifier::None,
                        }),
                    },
                ),
            ]),
        };

        // Write to notation
        let notation = write(&contract);

        // Parse back
        let parsed = parse(&notation).unwrap();

        // Verify structure
        match parsed {
            Contract::Object { properties, .. } => {
                assert!(properties.contains_key("name"));
                assert!(properties.contains_key("tags"));
            }
            _ => panic!("expected object"),
        }
    }
}
