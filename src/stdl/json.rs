#![cfg(feature = "stdl")]

#[cfg(feature = "stdl")]
use serde_json;
use crate::interpreter::Value;
use std::collections::HashMap;

pub fn handle(function: &str, args: Vec<Value>) -> Option<Result<Value, String>> {
    match function {
        "parse"     => Some(parse(args)),
        "stringify" => Some(stringify(args)),
        _ => None,
    }
}

pub fn is_type(name: &str) -> bool {
    false // no types, just functions
}

pub fn get_types() -> Vec<String> {
    vec![]
}

// JSON string -> Cigale Value
fn parse(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("parse() expects 1 argument but got {}", args.len()));
    }
    let s = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => return Err("parse() requires a string argument".to_string()),
    };
    let json: serde_json::Value = serde_json::from_str(&s)
        .map_err(|e| format!("JSON parse error: {}", e))?;
    Ok(json_to_value(json))
}

// Cigale Value -> JSON string
fn stringify(args: Vec<Value>) -> Result<Value, String> {
    if args.len() < 1 || args.len() > 2 {
        return Err(format!("stringify() expects 1 or 2 arguments but got {}", args.len()));
    }
    let val = &args[0];
    let pretty = match args.get(1) {
        Some(Value::Bool(b)) => *b,
        None => false,
        _ => return Err("stringify() second argument must be a bool".to_string()),
    };
    let json = value_to_json(val)?;
    let result = if pretty {
        serde_json::to_string_pretty(&json)
            .map_err(|e| format!("JSON stringify error: {}", e))?
    } else {
        serde_json::to_string(&json)
            .map_err(|e| format!("JSON stringify error: {}", e))?
    };
    Ok(Value::Str(result))
}

fn json_to_value(json: serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null          => Value::Null,
        serde_json::Value::Bool(b)       => Value::Bool(b),
        serde_json::Value::Number(n)     => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Double(f)
            } else {
                Value::Null
            }
        }
        serde_json::Value::String(s)     => Value::Str(s),
        serde_json::Value::Array(items)  => {
            Value::List(items.into_iter().map(json_to_value).collect())
        }
        serde_json::Value::Object(map)   => {
            let mut fields = HashMap::new();
            for (k, v) in map {
                fields.insert(k, json_to_value(v));
            }
            Value::Instance {
                class_name: "json_object".to_string(),
                parent: None,
                fields,
            }
        }
    }
}

fn value_to_json(val: &Value) -> Result<serde_json::Value, String> {
    match val {
        Value::Null        => Ok(serde_json::Value::Null),
        Value::Bool(b)     => Ok(serde_json::Value::Bool(*b)),
        Value::Int(n)      => Ok(serde_json::json!(*n)),
        Value::Float(f)    => Ok(serde_json::json!(*f as f64)),
        Value::Double(d)   => Ok(serde_json::json!(*d)),
        Value::Str(s)      => Ok(serde_json::Value::String(s.clone())),
        Value::List(items) => {
            let arr: Result<Vec<_>, _> = items.iter().map(value_to_json).collect();
            Ok(serde_json::Value::Array(arr?))
        }
        Value::Instance { fields, .. } => {
            let mut map = serde_json::Map::new();
            for (k, v) in fields {
                map.insert(k.clone(), value_to_json(v)?);
            }
            Ok(serde_json::Value::Object(map))
        }
        Value::EnumVariant(e, v) => Ok(serde_json::Value::String(format!("{}.{}", e, v))),
        _ => Err(format!("cannot stringify {:?} to JSON", val)),
    }
}