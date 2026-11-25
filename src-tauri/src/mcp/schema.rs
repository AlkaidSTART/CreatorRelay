use super::*;

pub(super) fn object_schema(properties: Value, required: Vec<&str>) -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": properties,
        "required": required
    })
}

pub(super) fn string_array_schema(max_items: usize) -> Value {
    json!({
        "type": "array",
        "minItems": 1,
        "maxItems": max_items,
        "items": { "type": "string", "minLength": 1, "maxLength": 128 }
    })
}

pub(super) fn media_ref_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "sourceContentId": { "type": "string", "minLength": 1, "maxLength": 128 },
            "assetId": { "type": "string", "minLength": 1, "maxLength": 128 },
            "order": { "type": "integer", "minimum": 1, "maximum": 999 },
            "caption": { "type": "string", "maxLength": 500 }
        },
        "required": ["sourceContentId", "assetId", "order"]
    })
}

pub(super) fn cover_ref_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "sourceContentId": { "type": "string", "minLength": 1, "maxLength": 128 },
            "assetId": { "type": "string", "minLength": 1, "maxLength": 128 }
        },
        "required": ["sourceContentId", "assetId"]
    })
}

pub(super) fn tool(
    name: &str,
    title: &str,
    description: &str,
    permission: ToolPermission,
    input_schema: Value,
) -> McpToolDefinition {
    McpToolDefinition {
        name: name.to_string(),
        title: title.to_string(),
        description: description.to_string(),
        schema_version: "1.0".to_string(),
        permission,
        input_schema,
    }
}


pub(super) fn normalized_timeout(value: Option<u64>) -> u64 {
    value
        .unwrap_or(DEFAULT_TIMEOUT_MS)
        .clamp(100, MAX_TIMEOUT_MS)
}

fn schema_type_matches(value: &Value, expected: &str) -> bool {
    match expected {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        _ => true,
    }
}

fn validate_schema(value: &Value, schema: &Value, path: &str, errors: &mut Vec<McpFieldError>) {
    if let Some(expected) = schema.get("type").and_then(Value::as_str)
        && !schema_type_matches(value, expected)
    {
        errors.push(McpFieldError {
            path: path.to_string(),
            message: format!("字段类型应为 {expected}"),
        });
        return;
    }
    if let Some(values) = schema.get("enum").and_then(Value::as_array)
        && !values.iter().any(|candidate| candidate == value)
    {
        errors.push(McpFieldError {
            path: path.to_string(),
            message: format!(
                "字段值必须是以下之一：{}",
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
    }
    if let Some(text) = value.as_str() {
        let length = text.chars().count() as i64;
        if let Some(min) = schema.get("minLength").and_then(Value::as_i64)
            && length < min
        {
            errors.push(McpFieldError {
                path: path.to_string(),
                message: format!("至少需要 {min} 个字符"),
            });
        }
        if let Some(max) = schema.get("maxLength").and_then(Value::as_i64)
            && length > max
        {
            errors.push(McpFieldError {
                path: path.to_string(),
                message: format!("不能超过 {max} 个字符"),
            });
        }
    }
    if let Some(number) = value.as_f64() {
        if let Some(minimum) = schema.get("minimum").and_then(Value::as_f64)
            && number < minimum
        {
            errors.push(McpFieldError {
                path: path.to_string(),
                message: format!("不能小于 {minimum}"),
            });
        }
        if let Some(maximum) = schema.get("maximum").and_then(Value::as_f64)
            && number > maximum
        {
            errors.push(McpFieldError {
                path: path.to_string(),
                message: format!("不能大于 {maximum}"),
            });
        }
    }
    if let Some(items) = value.as_array() {
        let length = items.len() as i64;
        if let Some(min) = schema.get("minItems").and_then(Value::as_i64)
            && length < min
        {
            errors.push(McpFieldError {
                path: path.to_string(),
                message: format!("至少需要 {min} 项"),
            });
        }
        if let Some(max) = schema.get("maxItems").and_then(Value::as_i64)
            && length > max
        {
            errors.push(McpFieldError {
                path: path.to_string(),
                message: format!("不能超过 {max} 项"),
            });
        }
        if let Some(item_schema) = schema.get("items") {
            for (index, item) in items.iter().enumerate() {
                validate_schema(item, item_schema, &format!("{path}[{index}]"), errors);
            }
        }
    }
    if let Some(object) = value.as_object() {
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for field in required.iter().filter_map(Value::as_str) {
                if !object.contains_key(field) {
                    errors.push(McpFieldError {
                        path: if path.is_empty() {
                            field.to_string()
                        } else {
                            format!("{path}.{field}")
                        },
                        message: "缺少必填字段".to_string(),
                    });
                }
            }
        }
        let properties = schema.get("properties").and_then(Value::as_object);
        for (key, child) in object {
            if let Some(property_schema) = properties.and_then(|values| values.get(key)) {
                let child_path = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{path}.{key}")
                };
                validate_schema(child, property_schema, &child_path, errors);
            } else if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
                errors.push(McpFieldError {
                    path: if path.is_empty() {
                        key.to_string()
                    } else {
                        format!("{path}.{key}")
                    },
                    message: "不允许的字段".to_string(),
                });
            }
        }
    }
}

pub(super) fn validate_tool_arguments(tool: &McpToolDefinition, arguments: &Value) -> Vec<McpFieldError> {
    let mut errors = Vec::new();
    validate_schema(arguments, &tool.input_schema, "", &mut errors);
    errors
}

