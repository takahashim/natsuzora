use crate::error::{NatsuzoraError, Result};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Integer range per spec: -9007199254740991 to 9007199254740991 (JavaScript safe integers)
pub const INTEGER_MIN: i64 = -9_007_199_254_740_991;
pub const INTEGER_MAX: i64 = 9_007_199_254_740_991;

/// Runtime value type for Natsuzora templates
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

impl Value {
    /// Convert a JSON value to a Natsuzora Value
    pub fn from_json(json: JsonValue) -> Result<Self> {
        match json {
            JsonValue::Null => Ok(Value::Null),
            JsonValue::Bool(b) => Ok(Value::Bool(b)),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    if i < INTEGER_MIN || i > INTEGER_MAX {
                        return Err(NatsuzoraError::TypeError {
                            message: format!("Integer out of range: {}", i),
                        });
                    }
                    Ok(Value::Integer(i))
                } else if let Some(f) = n.as_f64() {
                    // Try to convert float to integer if it's a whole number
                    if f.fract() == 0.0 && f >= INTEGER_MIN as f64 && f <= INTEGER_MAX as f64 {
                        Ok(Value::Integer(f as i64))
                    } else {
                        Err(NatsuzoraError::TypeError {
                            message: format!("Floating point numbers are not supported: {}", f),
                        })
                    }
                } else {
                    Err(NatsuzoraError::TypeError {
                        message: "Invalid number".to_string(),
                    })
                }
            }
            JsonValue::String(s) => Ok(Value::String(s)),
            JsonValue::Array(arr) => {
                let values: Result<Vec<Value>> = arr.into_iter().map(Value::from_json).collect();
                Ok(Value::Array(values?))
            }
            JsonValue::Object(obj) => {
                let mut map = HashMap::new();
                for (k, v) in obj {
                    map.insert(k, Value::from_json(v)?);
                }
                Ok(Value::Object(map))
            }
        }
    }

    /// Check if the value is truthy per spec section 3.4
    /// Falsy values: false, null, 0, "", [], {}
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Integer(n) => *n != 0,
            Value::String(s) => !s.is_empty(),
            Value::Array(arr) => !arr.is_empty(),
            Value::Object(obj) => !obj.is_empty(),
        }
    }

    /// Stringify the value per spec section 3.3
    /// Only String, Integer, and Null can be stringified
    pub fn stringify(&self) -> Result<String> {
        match self {
            Value::String(s) => Ok(s.clone()),
            Value::Integer(n) => {
                if *n < INTEGER_MIN || *n > INTEGER_MAX {
                    return Err(NatsuzoraError::TypeError {
                        message: format!("Integer out of range: {}", n),
                    });
                }
                Ok(n.to_string())
            }
            Value::Null => Ok(String::new()),
            Value::Bool(_) => Err(NatsuzoraError::TypeError {
                message: "Cannot stringify boolean value".to_string(),
            }),
            Value::Array(_) => Err(NatsuzoraError::TypeError {
                message: "Cannot stringify array".to_string(),
            }),
            Value::Object(_) => Err(NatsuzoraError::TypeError {
                message: "Cannot stringify object".to_string(),
            }),
        }
    }

    /// Ensure the value is an array and return a reference to it
    pub fn as_array(&self) -> Result<&Vec<Value>> {
        match self {
            Value::Array(arr) => Ok(arr),
            _ => Err(NatsuzoraError::TypeError {
                message: format!("Expected array, got {}", self.type_name()),
            }),
        }
    }

    /// Get the type name for error messages
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Integer(_) => "integer",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_truthy() {
        assert!(!Value::Null.is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Integer(0).is_truthy());
        assert!(Value::Integer(1).is_truthy());
        assert!(Value::Integer(-1).is_truthy());
        assert!(!Value::String("".to_string()).is_truthy());
        assert!(Value::String("hello".to_string()).is_truthy());
        assert!(!Value::Array(vec![]).is_truthy());
        assert!(Value::Array(vec![Value::Integer(1)]).is_truthy());
        assert!(!Value::Object(HashMap::new()).is_truthy());
    }

    #[test]
    fn test_stringify() {
        assert_eq!(
            Value::String("hello".to_string()).stringify().unwrap(),
            "hello"
        );
        assert_eq!(Value::Integer(42).stringify().unwrap(), "42");
        assert_eq!(Value::Integer(-42).stringify().unwrap(), "-42");
        assert_eq!(Value::Integer(0).stringify().unwrap(), "0");
        assert_eq!(Value::Null.stringify().unwrap(), "");

        assert!(Value::Bool(true).stringify().is_err());
        assert!(Value::Array(vec![]).stringify().is_err());
        assert!(Value::Object(HashMap::new()).stringify().is_err());
    }

    #[test]
    fn test_from_json() {
        let value = Value::from_json(json!({"name": "test", "count": 42})).unwrap();
        if let Value::Object(obj) = value {
            assert_eq!(obj.get("name"), Some(&Value::String("test".to_string())));
            assert_eq!(obj.get("count"), Some(&Value::Integer(42)));
        } else {
            panic!("Expected Object");
        }
    }
}
