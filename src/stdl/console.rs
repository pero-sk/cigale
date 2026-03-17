#[cfg(feature = "stdl")]
use crate::interpreter::Value;

pub fn handle(function: &str, args: Vec<Value>) -> Option<Result<Value, String>> {
    match function {
        "cout"  => Some(cout(args)),
        "couts" => Some(couts(args)),
        "cin"   => Some(cin(args)),
        "tostring" => Some(value_to_str(args)),
        _ => None,
    }
}

fn cout(args: Vec<Value>) -> Result<Value, String> {
    if args.is_empty() {
        return Ok(Value::Null);
    }

    // Expecting first argument to be a string
    let output = match &args[0] {
        Value::Str(s) => s,
        other => return Err(format!("cout() expects first argument to be string, got {:?}", other)),
    };

    println!("{}", output); // just the inner string
    Ok(Value::Null)
}

fn couts(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("couts() expects 1 argument but got {}", args.len()));
    }
    let output = match &args[0] {
        Value::Str(s) => s,
        other => return Err(format!("couts() expects first argument to be string, got {:?}", other)),
    };

    print!("{}", output); // just the inner string
    Ok(Value::Null)
}

fn cin(args: Vec<Value>) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!("cin() expects 0 arguments but got {}", args.len()));
    }
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)
        .map_err(|e| format!("cin error: {}", e))?;
    // trim both \r and \n
    Ok(Value::Str(input.trim_end_matches(|c| c == '\r' || c == '\n').to_string()))
}

fn value_to_str(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("tostring expects 1 argument but got {}", args.len()));
    }
    let val = &args[0];
    
    let s = match val {
        Value::Int(n)    => n.to_string(),
        Value::Float(f)  => f.to_string(),
        Value::Double(d) => d.to_string(),
        Value::Str(s)    => s.clone(),
        Value::Bool(b)   => b.to_string(),
        Value::Null      => "null".to_string(),
        Value::Ref(rc) => {
            match value_to_str(vec![rc.borrow().clone()])? {
                Value::Str(s) => s,
                _ => return Err("Expected string".to_string()),
            }
        }
        Value::EnumVariant(e, v) => format!("{}.{}", e, v),
        Value::List(items) => {
            let mut parts = Vec::new();
            for item in items {
                match value_to_str(vec![item.clone()])? {
                    Value::Str(s) => parts.push(s),
                    _ => return Err("Expected string from value_to_str".to_string()),
                }
            }
            format!("[{}]", parts.join(", "))
        },
        Value::Instance { class_name, .. } => format!("<{} instance>", class_name),
        Value::ResultVal { val, err } => {
            match val.as_ref() {
                Value::Null => {
                    let s = match value_to_str(vec![*err.clone()])? {
                        Value::Str(s) => s,
                        _ => return Err("Expected string".to_string()),
                    };
                    format!("err({})", s)
                }
                v => {
                    let s = match value_to_str(vec![v.clone()])? {
                        Value::Str(s) => s,
                        _ => return Err("Expected string".to_string()),
                    };
                    format!("ok({})", s)
                }
            }
        },
        Value::Function(f) => format!("<func {}>", f.name),
        Value::Identifier(n) => n.clone(),
        Value::NativeHandle(handle) => handle.get_name(),
    };

    Ok(Value::Str(s))
}