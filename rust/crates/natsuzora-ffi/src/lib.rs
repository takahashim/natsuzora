//! C FFI bindings for the Natsuzora template engine.
//!
//! Exposes `nz_render_json` and `nz_string_free` for use from Ruby (Fiddle) and other FFI consumers.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use natsuzora::error::NatsuzoraError;

/// Render a Natsuzora template with JSON data.
///
/// # Safety
///
/// - `template_utf8` must be a valid null-terminated UTF-8 string.
/// - `data_json_utf8` must be a valid null-terminated UTF-8 JSON string.
/// - `include_root_utf8_or_null` may be null, or a valid null-terminated UTF-8 string.
/// - `out_error_json_utf8` must be a valid pointer to a `*mut c_char` (initially null).
///
/// On success, returns a pointer to a null-terminated UTF-8 HTML string.
/// The caller must free it with `nz_string_free`.
///
/// On error, returns null and writes an error JSON string to `*out_error_json_utf8`.
/// The caller must free the error string with `nz_string_free`.
#[no_mangle]
pub unsafe extern "C" fn nz_render_json(
    template_utf8: *const c_char,
    data_json_utf8: *const c_char,
    include_root_utf8_or_null: *const c_char,
    out_error_json_utf8: *mut *mut c_char,
) -> *mut c_char {
    // Safety: caller guarantees valid pointers
    let template = match CStr::from_ptr(template_utf8).to_str() {
        Ok(s) => s,
        Err(e) => {
            write_error(out_error_json_utf8, "IoError", &e.to_string(), None, None);
            return ptr::null_mut();
        }
    };

    let data_json = match CStr::from_ptr(data_json_utf8).to_str() {
        Ok(s) => s,
        Err(e) => {
            write_error(out_error_json_utf8, "IoError", &e.to_string(), None, None);
            return ptr::null_mut();
        }
    };

    let include_root = if include_root_utf8_or_null.is_null() {
        None
    } else {
        match CStr::from_ptr(include_root_utf8_or_null).to_str() {
            Ok(s) => Some(s),
            Err(e) => {
                write_error(out_error_json_utf8, "IoError", &e.to_string(), None, None);
                return ptr::null_mut();
            }
        }
    };

    let data: serde_json::Value = match serde_json::from_str(data_json) {
        Ok(v) => v,
        Err(e) => {
            write_error(out_error_json_utf8, "IoError", &e.to_string(), None, None);
            return ptr::null_mut();
        }
    };

    let result = if let Some(root) = include_root {
        natsuzora::render_with_includes(template, data, root)
    } else {
        natsuzora::render(template, data)
    };

    match result {
        Ok(html) => match CString::new(html) {
            Ok(cs) => cs.into_raw(),
            Err(e) => {
                write_error(out_error_json_utf8, "IoError", &e.to_string(), None, None);
                ptr::null_mut()
            }
        },
        Err(err) => {
            write_natsuzora_error(out_error_json_utf8, &err);
            ptr::null_mut()
        }
    }
}

/// Free a string previously returned by `nz_render_json` or written to `out_error_json_utf8`.
///
/// # Safety
///
/// `p` must be a pointer previously returned by this crate via `CString::into_raw`,
/// or null (in which case this is a no-op).
#[no_mangle]
pub unsafe extern "C" fn nz_string_free(p: *mut c_char) {
    if !p.is_null() {
        drop(CString::from_raw(p));
    }
}

/// Convert a `NatsuzoraError` to error JSON and write it to the output pointer.
unsafe fn write_natsuzora_error(out: *mut *mut c_char, err: &NatsuzoraError) {
    let (error_type, message, line, column) = match err {
        NatsuzoraError::ParseError { message, location } => (
            "ParseError",
            message.clone(),
            Some(location.line),
            Some(location.column),
        ),
        NatsuzoraError::UndefinedVariable { message, location } => (
            "UndefinedVariable",
            message.clone(),
            Some(location.line),
            Some(location.column),
        ),
        NatsuzoraError::TypeError { message } => ("TypeError", message.clone(), None, None),
        NatsuzoraError::IncludeError { message } => ("IncludeError", message.clone(), None, None),
        NatsuzoraError::ShadowingError { name, origin } => (
            "ShadowingError",
            format!("Cannot shadow existing variable '{}' (already defined in {})", name, origin),
            None,
            None,
        ),
        NatsuzoraError::IoError(e) => ("IoError", e.to_string(), None, None),
    };

    write_error(out, error_type, &message, line, column);
}

/// Write an error JSON string to the output pointer.
unsafe fn write_error(
    out: *mut *mut c_char,
    error_type: &str,
    message: &str,
    line: Option<usize>,
    column: Option<usize>,
) {
    let json = serde_json::json!({
        "type": error_type,
        "message": message,
        "line": line,
        "column": column,
    });

    if let Ok(cs) = CString::new(json.to_string()) {
        *out = cs.into_raw();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_render_simple() {
        let template = CString::new("Hello, {[ name ]}!").unwrap();
        let data = CString::new(r#"{"name": "World"}"#).unwrap();
        let mut err_ptr: *mut c_char = ptr::null_mut();

        unsafe {
            let result =
                nz_render_json(template.as_ptr(), data.as_ptr(), ptr::null(), &mut err_ptr);
            assert!(!result.is_null(), "Expected non-null result");
            let html = CStr::from_ptr(result).to_str().unwrap();
            assert_eq!(html, "Hello, World!");
            nz_string_free(result);
        }
    }

    #[test]
    fn test_render_error() {
        let template = CString::new("{[ undefined_var ]}").unwrap();
        let data = CString::new(r#"{}"#).unwrap();
        let mut err_ptr: *mut c_char = ptr::null_mut();

        unsafe {
            let result =
                nz_render_json(template.as_ptr(), data.as_ptr(), ptr::null(), &mut err_ptr);
            assert!(result.is_null(), "Expected null result on error");
            assert!(!err_ptr.is_null(), "Expected error JSON");

            let err_json = CStr::from_ptr(err_ptr).to_str().unwrap();
            let err: serde_json::Value = serde_json::from_str(err_json).unwrap();
            assert_eq!(err["type"], "UndefinedVariable");
            assert!(err["message"]
                .as_str()
                .unwrap()
                .contains("undefined_var"));

            nz_string_free(err_ptr);
        }
    }

    #[test]
    fn test_render_parse_error() {
        let template = CString::new("{[#if]}missing condition{[/if]}").unwrap();
        let data = CString::new(r#"{}"#).unwrap();
        let mut err_ptr: *mut c_char = ptr::null_mut();

        unsafe {
            let result =
                nz_render_json(template.as_ptr(), data.as_ptr(), ptr::null(), &mut err_ptr);
            assert!(result.is_null(), "Expected null result on parse error");
            assert!(!err_ptr.is_null(), "Expected error JSON");

            let err_json = CStr::from_ptr(err_ptr).to_str().unwrap();
            let err: serde_json::Value = serde_json::from_str(err_json).unwrap();
            assert_eq!(err["type"], "ParseError");

            nz_string_free(err_ptr);
        }
    }

    #[test]
    fn test_render_html_escaping() {
        let template = CString::new("{[ html ]}").unwrap();
        let data = CString::new(r#"{"html": "<b>bold</b>"}"#).unwrap();
        let mut err_ptr: *mut c_char = ptr::null_mut();

        unsafe {
            let result =
                nz_render_json(template.as_ptr(), data.as_ptr(), ptr::null(), &mut err_ptr);
            assert!(!result.is_null());
            let html = CStr::from_ptr(result).to_str().unwrap();
            assert_eq!(html, "&lt;b&gt;bold&lt;/b&gt;");
            nz_string_free(result);
        }
    }

    #[test]
    fn test_string_free_null() {
        // Should be a no-op
        unsafe {
            nz_string_free(ptr::null_mut());
        }
    }

    #[test]
    fn test_invalid_json_data() {
        let template = CString::new("Hello").unwrap();
        let data = CString::new("not valid json").unwrap();
        let mut err_ptr: *mut c_char = ptr::null_mut();

        unsafe {
            let result =
                nz_render_json(template.as_ptr(), data.as_ptr(), ptr::null(), &mut err_ptr);
            assert!(result.is_null());
            assert!(!err_ptr.is_null());

            let err_json = CStr::from_ptr(err_ptr).to_str().unwrap();
            let err: serde_json::Value = serde_json::from_str(err_json).unwrap();
            assert_eq!(err["type"], "IoError");

            nz_string_free(err_ptr);
        }
    }

    #[test]
    fn test_null_value_error() {
        let template = CString::new("{[ value ]}").unwrap();
        let data = CString::new(r#"{"value": null}"#).unwrap();
        let mut err_ptr: *mut c_char = ptr::null_mut();

        unsafe {
            let result =
                nz_render_json(template.as_ptr(), data.as_ptr(), ptr::null(), &mut err_ptr);
            assert!(result.is_null());
            assert!(!err_ptr.is_null());

            let err_json = CStr::from_ptr(err_ptr).to_str().unwrap();
            let err: serde_json::Value = serde_json::from_str(err_json).unwrap();
            assert_eq!(err["type"], "TypeError");

            nz_string_free(err_ptr);
        }
    }

    #[test]
    fn test_type_error() {
        let template = CString::new("{[ value ]}").unwrap();
        let data = CString::new(r#"{"value": true}"#).unwrap();
        let mut err_ptr: *mut c_char = ptr::null_mut();

        unsafe {
            let result =
                nz_render_json(template.as_ptr(), data.as_ptr(), ptr::null(), &mut err_ptr);
            assert!(result.is_null());
            assert!(!err_ptr.is_null());

            let err_json = CStr::from_ptr(err_ptr).to_str().unwrap();
            let err: serde_json::Value = serde_json::from_str(err_json).unwrap();
            assert_eq!(err["type"], "TypeError");

            nz_string_free(err_ptr);
        }
    }
}
