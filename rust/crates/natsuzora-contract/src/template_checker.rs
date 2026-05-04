//! Template validation against contracts.
//!
//! Check if a template uses only fields defined in a contract.

use std::collections::BTreeMap;

use natsuzora::ast::{AstNode, IncludeLoader, Location, Path, Template};

use crate::types::Contract;

/// Error found when checking a template against a contract.
#[derive(Debug, Clone)]
pub struct TemplateCheckError {
    /// Location in the template source.
    pub location: Location,
    /// The path that caused the error.
    pub path: String,
    /// Description of the error.
    pub message: String,
    /// Suggestion for fixing the error (if available).
    pub suggestion: Option<String>,
}

impl std::fmt::Display for TemplateCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}: error: {}",
            self.location.line, self.location.column, self.message
        )?;
        if let Some(suggestion) = &self.suggestion {
            write!(f, " ({suggestion})")?;
        }
        Ok(())
    }
}

impl std::error::Error for TemplateCheckError {}

/// Check a template against a contract.
///
/// Returns a list of errors where the template uses fields not defined in the contract.
pub fn check_template<L: IncludeLoader>(
    template: &Template,
    contract: &Contract,
    loader: &mut L,
) -> Vec<TemplateCheckError> {
    let mut checker = TemplateChecker::new(contract, loader);
    checker.check_template(template);
    checker.errors
}

struct ScopeFrame<'a> {
    name: Option<String>,
    contract: &'a Contract,
}

struct TemplateChecker<'a, L> {
    root_contract: &'a Contract,
    #[allow(dead_code)] // Reserved for future include support
    loader: &'a mut L,
    #[allow(dead_code)] // Reserved for future include support
    include_stack: Vec<String>,
    errors: Vec<TemplateCheckError>,
}

impl<'a, L: IncludeLoader> TemplateChecker<'a, L> {
    fn new(contract: &'a Contract, loader: &'a mut L) -> Self {
        Self {
            root_contract: contract,
            loader,
            include_stack: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn check_template(&mut self, template: &Template) {
        let mut scopes = vec![ScopeFrame {
            name: None,
            contract: self.root_contract,
        }];
        self.check_nodes(template.nodes(), &mut scopes);
    }

    fn check_nodes(&mut self, nodes: &[AstNode], scopes: &mut Vec<ScopeFrame<'a>>) {
        for node in nodes {
            match node {
                AstNode::Text(_) => {}
                AstNode::Variable(var) => {
                    self.check_path(&var.path, scopes, PathUsage::Variable);
                }
                AstNode::Unsecure(unsecure) => {
                    self.check_path(&unsecure.path, scopes, PathUsage::Variable);
                }
                AstNode::If(block) => {
                    self.check_path(&block.condition, scopes, PathUsage::Condition);
                    self.check_nodes(&block.then_branch, scopes);
                    if let Some(else_branch) = &block.else_branch {
                        self.check_nodes(else_branch, scopes);
                    }
                }
                AstNode::Unless(block) => {
                    self.check_path(&block.condition, scopes, PathUsage::Condition);
                    self.check_nodes(&block.body, scopes);
                }
                AstNode::Each(block) => {
                    if let Some(items_contract) =
                        self.check_path(&block.collection, scopes, PathUsage::Each)
                    {
                        scopes.push(ScopeFrame {
                            name: Some(block.item_ident.clone()),
                            contract: items_contract,
                        });
                        self.check_nodes(&block.body, scopes);
                        scopes.pop();
                    }
                }
                AstNode::Include(include) => {
                    // Check include arguments - these can be any type
                    for arg in &include.args {
                        self.check_path(&arg.value, scopes, PathUsage::IncludeArg);
                    }

                    // Note: Full include checking would require the include's contract
                    // For now, we just verify the passed paths are valid
                }
            }
        }
    }

    fn check_path(
        &mut self,
        path: &Path,
        scopes: &[ScopeFrame<'a>],
        usage: PathUsage,
    ) -> Option<&'a Contract> {
        let segments = path.segments();
        if segments.is_empty() {
            return None;
        }

        // Find the starting scope
        let (start_contract, remainder) = self.resolve_scope(scopes, segments);

        // Walk the path through the contract
        let mut current = start_contract;
        let mut traversed = Vec::new();

        for segment in remainder.iter() {
            traversed.push(segment.as_str());

            match current {
                Contract::Object { properties, .. } => {
                    if let Some(child) = properties.get(segment) {
                        current = child;
                    } else {
                        // Field not found - generate error with suggestion
                        let suggestion = find_similar_field(properties, segment);
                        self.errors.push(TemplateCheckError {
                            location: path.location(),
                            path: path.as_str(),
                            message: format!("'{}' is not defined in contract", path.as_str()),
                            suggestion: suggestion.map(|s| format!("did you mean '{s}'?")),
                        });
                        return None;
                    }
                }
                Contract::Array { .. } => {
                    // Can't access properties on an array directly
                    self.errors.push(TemplateCheckError {
                        location: path.location(),
                        path: path.as_str(),
                        message: format!(
                            "cannot access property '{segment}' on array (use 'each' to iterate)"
                        ),
                        suggestion: None,
                    });
                    return None;
                }
                Contract::Scalar { .. } | Contract::Any => {
                    self.errors.push(TemplateCheckError {
                        location: path.location(),
                        path: path.as_str(),
                        message: format!(
                            "cannot access property '{segment}' on scalar value"
                        ),
                        suggestion: None,
                    });
                    return None;
                }
                Contract::TypeRef { name } => {
                    self.errors.push(TemplateCheckError {
                        location: path.location(),
                        path: path.as_str(),
                        message: format!("unresolved type reference '{name}'"),
                        suggestion: None,
                    });
                    return None;
                }
            }
        }

        // Check usage-specific constraints
        match usage {
            PathUsage::Variable => {
                // Variables should resolve to scalar
                match current {
                    Contract::Scalar { .. } | Contract::Any => {}
                    Contract::Object { .. } => {
                        self.errors.push(TemplateCheckError {
                            location: path.location(),
                            path: path.as_str(),
                            message: format!("'{}' is an object, not a scalar value", path.as_str()),
                            suggestion: Some("access a specific property".to_string()),
                        });
                    }
                    Contract::Array { .. } => {
                        self.errors.push(TemplateCheckError {
                            location: path.location(),
                            path: path.as_str(),
                            message: format!("'{}' is an array, not a scalar value", path.as_str()),
                            suggestion: Some("use 'each' to iterate".to_string()),
                        });
                    }
                    Contract::TypeRef { .. } => {}
                }
            }
            PathUsage::Condition => {
                // Conditions can be any type (truthiness check)
            }
            PathUsage::IncludeArg => {
                // Include arguments can be any type
            }
            PathUsage::Each => {
                // Each requires an array
                match current {
                    Contract::Array { items } => {
                        return Some(items.as_ref());
                    }
                    Contract::Any => {
                        // Any could be an array, allow it
                        return Some(current);
                    }
                    _ => {
                        self.errors.push(TemplateCheckError {
                            location: path.location(),
                            path: path.as_str(),
                            message: format!("'{}' is not an array", path.as_str()),
                            suggestion: None,
                        });
                        return None;
                    }
                }
            }
        }

        Some(current)
    }

    fn resolve_scope<'b>(
        &self,
        scopes: &'b [ScopeFrame<'a>],
        segments: &'b [String],
    ) -> (&'a Contract, &'b [String]) {
        let first = &segments[0];

        // Check scopes from innermost to outermost
        for frame in scopes.iter().rev() {
            if let Some(name) = &frame.name {
                if name == first {
                    return (frame.contract, &segments[1..]);
                }
            } else {
                // Root scope - return full path
                return (frame.contract, segments);
            }
        }

        // Fallback to root
        (self.root_contract, segments)
    }
}

#[derive(Clone, Copy)]
enum PathUsage {
    Variable,
    Condition,
    Each,
    IncludeArg, // Can be any type (passed to partial)
}

/// Find a similar field name for suggestions.
fn find_similar_field(properties: &BTreeMap<String, Contract>, target: &str) -> Option<String> {
    let target_lower = target.to_lowercase();

    // First, try exact case-insensitive match
    for key in properties.keys() {
        if key.to_lowercase() == target_lower {
            return Some(key.clone());
        }
    }

    // Then, try prefix match
    for key in properties.keys() {
        if key.to_lowercase().starts_with(&target_lower)
            || target_lower.starts_with(&key.to_lowercase())
        {
            return Some(key.clone());
        }
    }

    // Simple edit distance check for typos
    for key in properties.keys() {
        if levenshtein_distance(key, target) <= 2 {
            return Some(key.clone());
        }
    }

    None
}

/// Calculate Levenshtein distance between two strings.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

    for i in 0..=a_len {
        matrix[i][0] = i;
    }
    for j in 0..=b_len {
        matrix[0][j] = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[a_len][b_len]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContractModifier, LoaderError, ScalarType};
    use std::collections::BTreeMap;

    struct NoopLoader;

    impl IncludeLoader for NoopLoader {
        fn load(&mut self, _: &str) -> Result<natsuzora::ast::Template, LoaderError> {
            Err("no includes".into())
        }
    }

    fn make_contract() -> Contract {
        Contract::Object {
            required: vec!["site".into(), "posts".into()],
            properties: BTreeMap::from([
                (
                    "site".into(),
                    Contract::Object {
                        required: vec!["title".into()],
                        properties: BTreeMap::from([
                            (
                                "title".into(),
                                Contract::Scalar {
                                    scalar_type: ScalarType::Scalar,
                                    modifier: ContractModifier::None,
                                },
                            ),
                            (
                                "description".into(),
                                Contract::Scalar {
                                    scalar_type: ScalarType::Scalar,
                                    modifier: ContractModifier::Nullable,
                                },
                            ),
                        ]),
                    },
                ),
                (
                    "posts".into(),
                    Contract::Array {
                        items: Box::new(Contract::Object {
                            required: vec!["title".into()],
                            properties: BTreeMap::from([
                                (
                                    "title".into(),
                                    Contract::Scalar {
                                        scalar_type: ScalarType::Scalar,
                                        modifier: ContractModifier::None,
                                    },
                                ),
                                (
                                    "excerpt".into(),
                                    Contract::Scalar {
                                        scalar_type: ScalarType::Scalar,
                                        modifier: ContractModifier::Nullable,
                                    },
                                ),
                            ]),
                        }),
                    },
                ),
            ]),
        }
    }

    #[test]
    fn valid_template_passes() {
        let contract = make_contract();
        let template = natsuzora::ast::parse("{[ site.title ]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert!(errors.is_empty());
    }

    #[test]
    fn undefined_field_error() {
        let contract = make_contract();
        let template = natsuzora::ast::parse("{[ site.tagline ]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("not defined"));
        assert!(errors[0].path.contains("site.tagline"));
    }

    #[test]
    fn suggestion_for_typo() {
        let contract = make_contract();
        let template = natsuzora::ast::parse("{[ site.titel ]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].suggestion.as_ref().unwrap().contains("title"));
    }

    #[test]
    fn each_on_array_works() {
        let contract = make_contract();
        let template =
            natsuzora::ast::parse("{[#each posts as post]}{[ post.title ]}{[/each]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert!(errors.is_empty());
    }

    #[test]
    fn each_on_non_array_error() {
        let contract = make_contract();
        let template =
            natsuzora::ast::parse("{[#each site as item]}{[ item ]}{[/each]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("not an array"));
    }

    #[test]
    fn nested_path_in_each() {
        let contract = make_contract();
        let template =
            natsuzora::ast::parse("{[#each posts as post]}{[ post.unknown ]}{[/each]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("not defined"));
    }

    #[test]
    fn multiple_errors_in_template() {
        let contract = make_contract();
        let template = natsuzora::ast::parse("{[ site.foo ]}{[ site.bar ]}{[ unknown ]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn if_condition_allows_any_type() {
        let contract = make_contract();
        // site is an object, but can be used in if condition (truthiness check)
        let template = natsuzora::ast::parse("{[#if site]}has site{[/if]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert!(errors.is_empty());
    }

    #[test]
    fn unless_condition_allows_any_type() {
        let contract = make_contract();
        let template = natsuzora::ast::parse("{[#unless posts]}no posts{[/unless]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert!(errors.is_empty());
    }

    #[test]
    fn if_with_undefined_condition_error() {
        let contract = make_contract();
        let template = natsuzora::ast::parse("{[#if unknown]}test{[/if]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("not defined"));
    }

    #[test]
    fn object_as_scalar_error() {
        let contract = make_contract();
        let template = natsuzora::ast::parse("{[ site ]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("object"));
        assert!(errors[0].suggestion.is_some());
    }

    #[test]
    fn array_as_scalar_error() {
        let contract = make_contract();
        let template = natsuzora::ast::parse("{[ posts ]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("array"));
        assert!(errors[0].suggestion.as_ref().unwrap().contains("each"));
    }

    #[test]
    fn access_property_on_scalar_error() {
        let contract = make_contract();
        let template = natsuzora::ast::parse("{[ site.title.foo ]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("scalar"));
    }

    #[test]
    fn access_property_on_array_error() {
        let contract = make_contract();
        let template = natsuzora::ast::parse("{[ posts.title ]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("array"));
    }

    #[test]
    fn unsecure_output_check() {
        let contract = make_contract();
        let template = natsuzora::ast::parse("{[!unsecure site.title ]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert!(errors.is_empty());
    }

    #[test]
    fn unsecure_undefined_error() {
        let contract = make_contract();
        let template = natsuzora::ast::parse("{[!unsecure site.unknown ]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("not defined"));
    }

    #[test]
    fn include_args_allow_any_type() {
        let contract = make_contract();
        // Include arguments can be objects, arrays, or scalars
        let template =
            natsuzora::ast::parse("{[!include /header site=site posts=posts title=site.title ]}")
                .unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert!(errors.is_empty());
    }

    #[test]
    fn include_args_undefined_error() {
        let contract = make_contract();
        let template = natsuzora::ast::parse("{[!include /header data=unknown ]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("not defined"));
    }

    #[test]
    fn error_location_is_accurate() {
        let contract = make_contract();
        let template = natsuzora::ast::parse("Hello {[ site.unknown ]}!").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].location.line, 1);
        assert_eq!(errors[0].location.column, 10); // position of "site.unknown"
    }

    #[test]
    fn multiline_error_location() {
        let contract = make_contract();
        let template = natsuzora::ast::parse("Line 1\n{[ site.foo ]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].location.line, 2);
    }

    #[test]
    fn nested_each_scope() {
        // Create a contract with nested arrays
        let contract = Contract::Object {
            required: vec!["categories".into()],
            properties: BTreeMap::from([(
                "categories".into(),
                Contract::Array {
                    items: Box::new(Contract::Object {
                        required: vec!["name".into(), "posts".into()],
                        properties: BTreeMap::from([
                            (
                                "name".into(),
                                Contract::Scalar {
                                    scalar_type: ScalarType::Scalar,
                                    modifier: ContractModifier::None,
                                },
                            ),
                            (
                                "posts".into(),
                                Contract::Array {
                                    items: Box::new(Contract::Object {
                                        required: vec!["title".into()],
                                        properties: BTreeMap::from([(
                                            "title".into(),
                                            Contract::Scalar {
                                                scalar_type: ScalarType::Scalar,
                                                modifier: ContractModifier::None,
                                            },
                                        )]),
                                    }),
                                },
                            ),
                        ]),
                    }),
                },
            )]),
        };

        let template = natsuzora::ast::parse(
            "{[#each categories as cat]}{[ cat.name ]}{[#each cat.posts as post]}{[ post.title ]}{[/each]}{[/each]}",
        )
        .unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert!(errors.is_empty());
    }

    #[test]
    fn nested_each_undefined_in_inner() {
        let contract = Contract::Object {
            required: vec!["categories".into()],
            properties: BTreeMap::from([(
                "categories".into(),
                Contract::Array {
                    items: Box::new(Contract::Object {
                        required: vec!["posts".into()],
                        properties: BTreeMap::from([(
                            "posts".into(),
                            Contract::Array {
                                items: Box::new(Contract::Object {
                                    required: vec!["title".into()],
                                    properties: BTreeMap::from([(
                                        "title".into(),
                                        Contract::Scalar {
                                            scalar_type: ScalarType::Scalar,
                                            modifier: ContractModifier::None,
                                        },
                                    )]),
                                }),
                            },
                        )]),
                    }),
                },
            )]),
        };

        let template = natsuzora::ast::parse(
            "{[#each categories as cat]}{[#each cat.posts as post]}{[ post.unknown ]}{[/each]}{[/each]}",
        )
        .unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].path.contains("post.unknown"));
    }

    #[test]
    fn if_else_both_branches_checked() {
        let contract = make_contract();
        let template =
            natsuzora::ast::parse("{[#if site]}{[ site.foo ]}{[#else]}{[ site.bar ]}{[/if]}")
                .unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert_eq!(errors.len(), 2); // Both foo and bar are undefined
    }

    #[test]
    fn case_insensitive_suggestion() {
        let contract = make_contract();
        let template = natsuzora::ast::parse("{[ site.Title ]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].suggestion.as_ref().unwrap().contains("title"));
    }

    #[test]
    fn levenshtein_suggestion() {
        let contract = make_contract();
        // "tite" has edit distance 1 from "title"
        let template = natsuzora::ast::parse("{[ site.tite ]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].suggestion.as_ref().unwrap().contains("title"));
    }

    #[test]
    fn no_suggestion_for_unrelated_name() {
        let contract = make_contract();
        let template = natsuzora::ast::parse("{[ site.xyz ]}").unwrap();
        let mut loader = NoopLoader;
        let errors = check_template(&template, &contract, &mut loader);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].suggestion.is_none());
    }

    #[test]
    fn display_format() {
        let error = TemplateCheckError {
            location: Location::new(10, 5, 100),
            path: "site.unknown".to_string(),
            message: "'site.unknown' is not defined".to_string(),
            suggestion: Some("did you mean 'title'?".to_string()),
        };
        let display = format!("{error}");
        assert!(display.contains("10:5"));
        assert!(display.contains("not defined"));
        assert!(display.contains("did you mean"));
    }
}
