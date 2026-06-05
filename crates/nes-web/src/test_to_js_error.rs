use wasm_bindgen::JsValue;

fn to_js_error(err: String) -> JsValue {
    JsValue::from_str(&err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_js_error_preserves_string() {
        let js_val = to_js_error("Custom error".to_string());
        assert_eq!(js_val.as_string(), Some("Custom error".to_string()));
    }
}
