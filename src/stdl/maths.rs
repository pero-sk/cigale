#[cfg(feature = "stdl")]
use crate::interpreter::Value;



pub fn handle(function: &str, args: Vec<Value>) -> Option<Result<Value, String>> {
    match function {
        _ => None,
    }
}

pub fn get_value(name: &str) -> Option<Value> {
    match name {
        "pi"  => Some(Value::Double(std::f64::consts::PI)),
        "e"   => Some(Value::Double(std::f64::consts::E)),
        "tau" => Some(Value::Double(std::f64::consts::TAU)),
        _     => None,
    }
}
