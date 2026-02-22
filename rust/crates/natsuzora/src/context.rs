//! Context for variable resolution during template rendering.

use crate::error::{Location, NatsuzoraError, Result};
use crate::value::Value;
use std::collections::HashMap;

/// Context for variable resolution during template rendering
pub struct Context {
    root: HashMap<String, Value>,
    local_stack: Vec<HashMap<String, Value>>,
}

impl Context {
    /// Create a new context from root data
    pub fn new(root_data: Value) -> Result<Self> {
        let root = match root_data {
            Value::Object(obj) => obj,
            _ => {
                return Err(NatsuzoraError::TypeError {
                    message: "Root data must be an object".to_string(),
                });
            }
        };

        Ok(Self {
            root,
            local_stack: Vec::new(),
        })
    }

    /// Resolve a path (e.g., ["user", "profile", "name"]) with location for error reporting
    pub fn resolve(&self, path: &[String], location: Location) -> Result<&Value> {
        let name = path
            .first()
            .ok_or_else(|| NatsuzoraError::UndefinedVariable {
                name: "<empty path>".to_string(),
                location,
            })?;

        let mut value = self.resolve_name(name, location)?;

        for segment in &path[1..] {
            value = self.access_property(value, segment, location)?;
        }

        Ok(value)
    }

    /// Push a new scope (for each blocks) with shadowing validation
    pub fn push_scope(&mut self, bindings: HashMap<String, Value>) -> Result<()> {
        self.validate_no_shadowing(&bindings)?;
        self.local_stack.push(bindings);
        Ok(())
    }

    /// Push scope for include (no shadowing validation per spec)
    pub fn push_include_scope(&mut self, bindings: HashMap<String, Value>) {
        self.local_stack.push(bindings);
    }

    /// Pop the current scope
    pub fn pop_scope(&mut self) {
        self.local_stack.pop();
    }

    /// Resolve a name from the scope stack or root
    fn resolve_name(&self, name: &str, location: Location) -> Result<&Value> {
        // Search local scopes from innermost to outermost
        for scope in self.local_stack.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Ok(value);
            }
        }

        // Fall back to root
        self.root
            .get(name)
            .ok_or_else(|| NatsuzoraError::UndefinedVariable {
                name: name.to_string(),
                location,
            })
    }

    /// Validate that bindings don't shadow existing names
    fn validate_no_shadowing(&self, bindings: &HashMap<String, Value>) -> Result<()> {
        for name in bindings.keys() {
            if self.name_exists(name) {
                return Err(NatsuzoraError::ShadowingError {
                    name: name.to_string(),
                });
            }
        }
        Ok(())
    }

    /// Check if a name exists in any scope
    fn name_exists(&self, name: &str) -> bool {
        for scope in &self.local_stack {
            if scope.contains_key(name) {
                return true;
            }
        }
        self.root.contains_key(name)
    }

    /// Get the length of an array at a path (without holding a reference)
    pub fn get_array_len(&self, path: &[String], location: Location) -> Result<usize> {
        let value = self.resolve(path, location)?;
        match value {
            Value::Array(arr) => Ok(arr.len()),
            _ => Err(NatsuzoraError::TypeError {
                message: format!("Expected array, got {}", value.type_name()),
            }),
        }
    }

    /// Get and clone a single array item by index (without holding a reference)
    pub fn get_array_item(
        &self,
        path: &[String],
        index: usize,
        location: Location,
    ) -> Result<Value> {
        let value = self.resolve(path, location)?;
        match value {
            Value::Array(arr) => arr
                .get(index)
                .cloned()
                .ok_or_else(|| NatsuzoraError::TypeError {
                    message: format!("Array index {} out of bounds", index),
                }),
            _ => Err(NatsuzoraError::TypeError {
                message: format!("Expected array, got {}", value.type_name()),
            }),
        }
    }

    /// Access a property on an object value
    fn access_property<'a>(
        &self,
        value: &'a Value,
        key: &str,
        location: Location,
    ) -> Result<&'a Value> {
        match value {
            Value::Object(obj) => obj
                .get(key)
                .ok_or_else(|| NatsuzoraError::UndefinedVariable {
                    name: key.to_string(),
                    location,
                }),
            _ => Err(NatsuzoraError::TypeError {
                message: format!("Cannot access property '{}' on non-object", key),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_location() -> Location {
        Location::new(1, 1, 0)
    }

    fn create_test_context() -> Context {
        let mut root = HashMap::new();
        root.insert("name".to_string(), Value::String("Alice".to_string()));

        let mut user = HashMap::new();
        user.insert(
            "email".to_string(),
            Value::String("alice@example.com".to_string()),
        );
        root.insert("user".to_string(), Value::Object(user));

        Context {
            root,
            local_stack: Vec::new(),
        }
    }

    #[test]
    fn test_resolve_simple() {
        let ctx = create_test_context();
        let value = ctx.resolve(&["name".to_string()], test_location()).unwrap();
        assert_eq!(value, &Value::String("Alice".to_string()));
    }

    #[test]
    fn test_resolve_path() {
        let ctx = create_test_context();
        let value = ctx
            .resolve(&["user".to_string(), "email".to_string()], test_location())
            .unwrap();
        assert_eq!(value, &Value::String("alice@example.com".to_string()));
    }

    #[test]
    fn test_undefined_variable() {
        let ctx = create_test_context();
        let result = ctx.resolve(&["unknown".to_string()], test_location());
        assert!(result.is_err());
    }

    #[test]
    fn test_scope_stack() {
        let mut ctx = create_test_context();
        let mut bindings = HashMap::new();
        bindings.insert("item".to_string(), Value::Integer(42));
        ctx.push_scope(bindings).unwrap();

        let value = ctx.resolve(&["item".to_string()], test_location()).unwrap();
        assert_eq!(value, &Value::Integer(42));

        ctx.pop_scope();
        assert!(ctx.resolve(&["item".to_string()], test_location()).is_err());
    }

    #[test]
    fn test_shadowing_error() {
        let mut ctx = create_test_context();
        let mut bindings = HashMap::new();
        bindings.insert("name".to_string(), Value::String("Bob".to_string()));

        let result = ctx.push_scope(bindings);
        assert!(matches!(result, Err(NatsuzoraError::ShadowingError { .. })));
    }

    #[test]
    fn test_include_scope_allows_shadowing() {
        let mut ctx = create_test_context();
        let mut bindings = HashMap::new();
        bindings.insert("name".to_string(), Value::String("Bob".to_string()));

        ctx.push_include_scope(bindings);
        let value = ctx.resolve(&["name".to_string()], test_location()).unwrap();
        assert_eq!(value, &Value::String("Bob".to_string()));
    }
}
