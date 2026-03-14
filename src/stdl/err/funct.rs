#[cfg(feature = "stdl")]
use crate::interpreter::Value;
use super::{make_ok_result, make_err_result};

pub fn handle(function: &str, args: Vec<Value>) -> Option<Result<Value, String>> {
    match function {
        "ok"  => Some(ok(args)),
        "err" => Some(err(args)),
        _ => None,
    }
}

fn ok(args: Vec<Value>) -> Result<Value, String> {
    let val = args.into_iter().next().unwrap_or(Value::Null);
    Ok(make_ok_result(val))
}

fn inherits_from_error(val: &Value) -> bool {
    match val {
        Value::Instance { class_name, parent, .. } => {
            if class_name == "Error" { return true; }
            match parent {
                Some(p) => inherits_from_error_by_name(p),
                None => false,
            }
        }
        _ => false,
    }
}

fn inherits_from_error_by_name(name: &str) -> bool {
    // since we only have the name here, just check if it's Error
    name == "Error"
}

fn err(args: Vec<Value>) -> Result<Value, String> {
    let val = args.into_iter().next().unwrap_or(Value::Null);
    match &val {
        Value::Str(s) => Ok(make_err_result(s.clone())),
        Value::Instance { .. } => {
            if inherits_from_error(&val) {
                Ok(Value::ResultVal {
                    val: Box::new(Value::Null),
                    err: Box::new(val),
                })
            } else {
                Err(format!("err() requires an Error instance — class must inherit from Error"))
            }
        }
        _ => Err("err() requires a string or Error instance".to_string()),
    }
}