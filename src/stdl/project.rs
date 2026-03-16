use crate::interpreter::Value;
use std::collections::HashMap;

fn parse_project_cfg() -> HashMap<String, String> {
    let mut map = HashMap::new();

    // look for project.cfg in cwd
    let content = match std::fs::read_to_string("project.cfg") {
        Ok(s) => s,
        Err(_) => return map,
    };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        if let Some((key, val)) = line.split_once('=') {
            let key = key.trim().to_string();
            let val = val.trim()
                .trim_end_matches(';')
                .trim()
                .trim_matches('"')
                .to_string();
            map.insert(key, val);
        }
    }
    map
}

pub fn get_value(name: &str) -> Option<Value> {
    let cfg = parse_project_cfg();
    match name {
        "name"        => Some(Value::Str(cfg.get("name").cloned().unwrap_or_default())),
        "description" => Some(Value::Str(cfg.get("description").cloned().unwrap_or_default())),
        "version"     => Some(Value::Str(cfg.get("version").cloned().unwrap_or_default())),
        _ => None,
    }
}

pub fn handle(function: &str, _args: Vec<Value>) -> Option<Result<Value, String>> {
    None // no functions, only values
}

pub fn is_type(_name: &str) -> bool {
    false
}

pub fn get_types() -> Vec<String> {
    vec![]
}