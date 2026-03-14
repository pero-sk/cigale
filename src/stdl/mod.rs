#![allow(unused)]
#[cfg(feature = "stdl")]
pub mod console;
pub mod io;
pub mod err;
pub mod maths;
pub mod project;
pub mod json;

use crate::interpreter::Value;

pub fn handle(module: &[String], function: &str, args: Vec<Value>) -> Option<Result<Value, String>> {
    // module is the full path e.g. ["stdl", "console"]
    // we match on the last part
    match module.last().map(|s| s.as_str()) {
        Some("console") => console::handle(function, args),
        Some("io")      => io::handle(function, args),
        Some("err")     => err::handle(function, args),
        Some("funct")   => err::funct::handle(function, args),
        Some("maths")   => maths::handle(function, args),
        Some("json")    => json::handle(function, args),
        // Some("project") => project::handle(function, args),
        _ => None,
    }
}
// for stdl variables like pi, name, description
pub fn get_value(module: &[String], name: &str) -> Option<Value> {
    match module.last().map(|s| s.as_str()) {
        Some("maths")   => maths::get_value(name),
        // Some("project") => project::get_value(name),
        _ => None,
    }
}

pub fn is_type(module: &[String], name: &str) -> bool {
    match module.last().map(|s| s.as_str()) {
        Some("io")  => io::is_type(name),
        Some("err") => err::is_type(name),
        _ => false,
    }
}

// returns all types exported by a module (for wildcard imports)
pub fn get_types(module: &[String]) -> Option<Vec<String>> {
    match module.last().map(|s| s.as_str()) {
        Some("io")  => Some(io::get_types()),
        Some("err") => Some(err::get_types()),
        _ => None,
    }
}

pub fn get_enum_variants(module: &[String], name: &str) -> Option<Vec<String>> {
    match module.last().map(|s| s.as_str()) {
        Some("io")  => io::get_enum_variants(name),
        _ => None,
    }
}