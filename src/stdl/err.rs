#![allow(unused)]
#[cfg(feature = "stdl")]
use std::collections::HashMap;
use crate::interpreter::Value;
pub mod funct;

pub fn handle(function: &str, args: Vec<Value>) -> Option<Result<Value, String>> {
    match function {
        "Error" => Some(make_error_instance(args)),
        _ => None,
    }
}

fn make_error_instance(args: Vec<Value>) -> Result<Value, String> {
    let msg = args.into_iter().next().unwrap_or(Value::Null);
    Ok(Value::Instance {
        class_name: "Error".to_string(),
        fields: {
            let mut m = HashMap::new();
            m.insert("msg".to_string(), msg);
            m
        },
    })
}



pub fn is_type(name: &str) -> bool {
    matches!(name, "result" | "Error")
}

pub fn get_types() -> Vec<String> {
    vec!["result".to_string(), "Error".to_string()]
}

pub fn make_ok_result(val: Value) -> Value {
    Value::ResultVal {
        val: Box::new(val),
        err: Box::new(Value::Null),
    }
}

pub fn make_err_result(msg: String) -> Value {
    Value::ResultVal {
        val: Box::new(Value::Null),
        err: Box::new(make_error(msg)),
    }
}

fn make_error(msg: String) -> Value {
    Value::Instance {
        class_name: "Error".to_string(),
        fields: {
            let mut m = HashMap::new();
            m.insert("msg".to_string(), Value::Str(msg));
            m
        },
    }
}