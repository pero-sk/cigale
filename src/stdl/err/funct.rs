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

fn err(args: Vec<Value>) -> Result<Value, String> {
    let val = args.into_iter().next().unwrap_or(Value::Null);
    // err() takes either a string message or an Error instance
    match val {
        Value::Str(s) => Ok(make_err_result(s)),
        Value::Instance { ref class_name, .. } if class_name == "Error" => {
            Ok(Value::ResultVal {
                val: Box::new(Value::Null),
                err: Box::new(val),
            })
        }
        _ => Err("err() requires a string or Error instance".to_string()),
    }
}