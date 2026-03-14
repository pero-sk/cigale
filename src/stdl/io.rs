#![allow(unused)]
#[cfg(feature = "stdl")]
use std::sync::{Arc, Mutex};
use std::io::{Read, Write, Seek, SeekFrom};
use std::fs::{File, OpenOptions};
use std::collections::HashMap;
use crate::interpreter::Value;

#[derive(Debug)]
pub struct FileHandle {
    pub path: String,
    pub perm: String,
    pub file: Option<File>,
    pub closed: bool,
}

impl FileHandle {
    fn new(path: String, perm: String, file: File) -> Self {
        FileHandle { path, perm, file: Some(file), closed: false }
    }
}

pub fn handle(function: &str, args: Vec<Value>) -> Option<Result<Value, String>> {
    match function {
        "open" => Some(open(args)),
        _ => None,
    }
}

fn make_error(msg: String) -> Value {
    Value::Instance {
        class_name: "Error".to_string(),
        parent: None,
        fields: {
            let mut m = HashMap::new();
            m.insert("msg".to_string(), Value::Str(msg));
            m
        },
    }
}

fn make_err_result(msg: String) -> Value {
    Value::ResultVal {
        val: Box::new(Value::Null),
        err: Box::new(make_error(msg)),
    }
}

fn make_ok_result(val: Value) -> Value {
    Value::ResultVal {
        val: Box::new(val),
        err: Box::new(Value::Null),
    }
}

fn open(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!("open() expects 2 arguments but got {}", args.len()));
    }

    let path = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => return Err("open() first argument must be a string path".to_string()),
    };

    let perm = match &args[1] {
        Value::EnumVariant(_, p) => p.clone(),
        _ => return Err("open() second argument must be a perm variant".to_string()),
    };

    let file_exists = std::path::Path::new(&path).exists();

    if !file_exists && !perm.contains('P') {
        return Ok(make_err_result(format!("file not found: {}", path)));
    }

    // build OpenOptions from perm
    let file = match open_with_perm(&path, &perm) {
        Ok(f) => f,
        Err(e) => return Ok(make_err_result(format!("failed to open file: {}", e))),
    };

    let handle = FileHandle::new(path, perm, file);
    Ok(make_ok_result(Value::NativeHandle(
        crate::interpreter::NativeHandle::File(
            Arc::new(Mutex::new(handle))
        )
    )))
}

fn open_with_perm(path: &str, perm: &str) -> std::io::Result<File> {
    let mut opts = OpenOptions::new();
    if perm.contains('R') { opts.read(true); }
    if perm.contains('W') { opts.write(true); }
    if perm.contains('A') { opts.append(true); }
    if perm.contains('P') { opts.create(true); }
    // if write or append but no P, don't truncate
    if perm.contains('W') && !perm.contains('A') {
        opts.truncate(false);
    }
    opts.open(path)
}

pub fn file_method(handle: Arc<Mutex<FileHandle>>, method: &str, args: Vec<Value>) -> Result<Value, String> {
    match method {
        "read"   => file_read(handle),
        "write"  => file_write(handle, args),
        "append" => file_append(handle, args),
        "close"  => file_close(handle),
        _ => Err(format!("file has no method {}", method)),
    }
}

fn file_read(handle: Arc<Mutex<FileHandle>>) -> Result<Value, String> {
    let mut h = handle.lock().map_err(|e| format!("file lock error: {}", e))?;
    if h.closed {
        return Ok(make_err_result("file is closed".to_string()));
    }
    if !h.perm.contains('R') {
        return Ok(make_err_result("no read permission".to_string()));
    }
    let file = match h.file.as_mut() {
        Some(f) => f,
        None => return Ok(make_err_result("file handle is invalid".to_string())),
    };
    // seek to start before reading
    file.seek(SeekFrom::Start(0))
        .map_err(|e| format!("seek error: {}", e))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("read error: {}", e))?;
    Ok(make_ok_result(Value::Str(content)))
}

fn file_write(handle: Arc<Mutex<FileHandle>>, args: Vec<Value>) -> Result<Value, String> {
    let mut h = handle.lock().map_err(|e| format!("file lock error: {}", e))?;
    if h.closed {
        return Ok(make_err_result("file is closed".to_string()));
    }
    if !h.perm.contains('W') {
        return Ok(make_err_result("no write permission".to_string()));
    }
    let content = match args.as_slice() {
        [Value::Str(s)] => s.clone(),
        _ => return Err("write() requires a string argument".to_string()),
    };
    let file = match h.file.as_mut() {
        Some(f) => f,
        None => return Ok(make_err_result("file handle is invalid".to_string())),
    };
    // seek to start before writing
    file.seek(SeekFrom::Start(0))
        .map_err(|e| format!("seek error: {}", e))?;
    let _ = match file.write_all(content.as_bytes()) {
        Ok(_)  => Ok::<Value, String>(make_ok_result(Value::Bool(true))),
        Err(e) => Ok::<Value, String>(make_err_result(e.to_string())),
    };
    Ok(make_ok_result(Value::Bool(true)))
}

fn file_append(handle: Arc<Mutex<FileHandle>>, args: Vec<Value>) -> Result<Value, String> {
    let mut h = handle.lock().map_err(|e| format!("file lock error: {}", e))?;
    if h.closed {
        return Ok(make_err_result("file is closed".to_string()));
    }
    if !h.perm.contains('A') {
        return Ok(make_err_result("no append permission".to_string()));
    }
    let content = match args.as_slice() {
        [Value::Str(s)] => s.clone(),
        _ => return Err("append() requires a string argument".to_string()),
    };
    let file = match h.file.as_mut() {
        Some(f) => f,
        None => return Ok(make_err_result("file handle is invalid".to_string())),
    };
    // seek to end before appending
    file.seek(SeekFrom::End(0))
        .map_err(|e| format!("seek error: {}", e))?;
    let _ = match file.write_all(content.as_bytes()) {
        Ok(_)  => Ok::<Value, String>(make_ok_result(Value::Bool(true))),
        Err(e) => Ok::<Value, String>(make_err_result(e.to_string())),
    };
    Ok(make_ok_result(Value::Bool(true)))
}

fn file_close(handle: Arc<Mutex<FileHandle>>) -> Result<Value, String> {
    let mut h = handle.lock().map_err(|e| format!("file lock error: {}", e))?;
    if h.closed {
        return Ok(make_err_result("file is already closed".to_string()));
    }
    // drop the file handle to close it
    h.file = None;
    h.closed = true;
    Ok(Value::Null)
}

pub fn is_type(name: &str) -> bool {
    matches!(name, "file" | "perm")
}

pub fn get_types() -> Vec<String> {
    vec!["file".to_string(), "perm".to_string()]
}

pub fn get_enum_variants(name: &str) -> Option<Vec<String>> {
    match name {
        "perm" => Some(vec![
            "R".to_string(),
            "W".to_string(),
            "P".to_string(),
            "A".to_string(),
            "RW".to_string(),
            "RP".to_string(),
            "RA".to_string(),
            "WP".to_string(),
            "WA".to_string(),
            "PA".to_string(),
            "RWP".to_string(),
            "RWA".to_string(),
            "RPA".to_string(),
            "WPA".to_string(),
            "RWPA".to_string(),
        ]),
        _ => None,
    }
}