use std::cell::RefCell;
use std::collections::HashMap;
#[cfg(feature = "stdl")]
use std::collections::HashSet;
use std::ops::Deref;
use std::rc::Rc;
#[cfg(feature = "stdl")]
use std::sync::Mutex;
use crate::analyser::type_to_str;
use crate::parser::ast::BinaryOp;
use crate::parser::ast::Expr;
use crate::parser::ast::FunctionDecl;
use crate::parser::ast::ClassDecl;
use crate::parser::ast::Program;
use crate::parser::ast::Stmt;
use crate::parser::ast::StringPart;
use crate::parser::ast::Type;
use crate::parser::ast::Literal;
use crate::parser::ast::UnaryOp;
#[derive(Debug, Clone)]
pub enum Signal {
    Return(Value),
    Break,
    Continue,
}

#[derive(Debug)]
pub enum ExecError {
    Signal(Signal),
    Error(String),
}

impl From<String> for ExecError {
    fn from(s: String) -> Self {
        ExecError::Error(s)
    }
}

#[derive(Clone)]
pub enum NativeHandle {
    #[cfg(feature = "stdl")]
    File(std::sync::Arc<std::sync::Mutex<crate::stdl::io::FileHandle>>),
    #[cfg(not(feature = "stdl"))]
    File(),
}
impl std::fmt::Debug for NativeHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "stdl")]
            NativeHandle::File(h) => {
                match h.lock() {
                    Ok(h) => {
                        if h.closed {
                            write!(f, "<file {} [closed]>", h.path)
                        } else {
                            write!(f, "<file {} [{}]>", h.path, h.perm)
                        }
                    }
                    Err(_) => write!(f, "<file [locked]>"),
                }
            }
            #[cfg(not(feature = "stdl"))]
            NativeHandle::File() => write!(f, "<native handle — enable stdl feature to use>"),
        }
    }
}
impl NativeHandle {
    pub fn get_name(&self) -> String {
        match self {
            #[cfg(feature = "stdl")]
            NativeHandle::File(h) => {
                match h.lock() {
                    Ok(h) => {
                        if h.closed {
                            format!("<file {} [closed]>", h.path)
                        } else {
                            format!("<file {} [{}]>", h.path, h.perm)
                        }
                    }
                    Err(_) => "<file [locked]>".to_string(),
                }
            }
            #[cfg(not(feature = "stdl"))]
            _ => {
                {
                    "<native handle — enable stdl feature to use>".to_string()
                }
            }
        }
    }
}


#[derive(Debug, Clone)]

pub enum Value {
    Int(i64),
    Float(f32),
    Double(f64),
    Str(String),
    Bool(bool),
    List(Vec<Value>),
    EnumVariant(String, String),
    Instance {
        class_name: String,
        fields: HashMap<String, Value>,
        parent: Option<String>,
    },
    ResultVal {
        val: Box<Value>,
        err: Box<Value>,
    },
    NativeHandle(NativeHandle),
    Function(FunctionDecl),
    Identifier(String),
    Ref(std::rc::Rc<std::cell::RefCell<Value>>),
    Null,
}

struct Environment {
    vars: HashMap<String, Value>,
    parent: Option<Box<Environment>>,
}
pub struct Interpreter {
    env: Environment,
    classes: HashMap<String, ClassDecl>,
    refs: HashMap<String, Rc<RefCell<Value>>>,
    enums: HashMap<String, Vec<String>>,  // name -> variants
    pub base_dir: String,
    #[cfg(feature = "stdl")]
    imports: HashMap<String, Vec<String>>,
    #[cfg(feature = "stdl")]
    registered_types: std::collections::HashSet<String>,
    #[cfg(feature = "stdl")]
    pub self_ref: Option<std::sync::Weak<Mutex<Interpreter>>>,
}

impl Environment {
    fn new() -> Self {
        Environment {
            vars: HashMap::new(),
            parent: None,
        }
    }

    fn new_child(parent: Environment) -> Self {
        Environment {
            vars: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    fn get(&self, name: &str) -> Option<&Value> {
        self.vars.get(name).or_else(|| {
            self.parent.as_ref()?.get(name)
        })
    }

    fn set(&mut self, name: &str, value: Value) {
        self.vars.insert(name.to_string(), value);
    }

    // assign to existing variable, walking up scope chain
    fn assign(&mut self, name: &str, value: Value) -> bool {
        if self.vars.contains_key(name) {
            self.vars.insert(name.to_string(), value);
            true
        } else if let Some(parent) = self.parent.as_mut() {
            parent.assign(name, value)
        } else {
            false // variable not found anywhere
        }
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            env: Environment::new(),
            classes: HashMap::new(),
            refs: HashMap::new(),
            enums: HashMap::new(),
            base_dir: ".".to_string(),
            #[cfg(feature = "stdl")]
            imports: HashMap::new(),
            #[cfg(feature = "stdl")]
            registered_types: HashSet::new(),
            #[cfg(feature = "stdl")]
            self_ref: None,
        }
    }

    pub fn run(&mut self, program: Program) -> Result<(), String> {
        for stmt in &program.body {
            match stmt {
                Stmt::FunctionDeclaration(f) => {
                    self.env.set(&f.name, Value::Function(f.clone()));
                }
                Stmt::ClassDeclaration(c) => {
                    self.classes.insert(c.name.clone(), c.clone());
                }
                Stmt::EnumDeclaration { name, variants } => {
                    self.enums.insert(name.clone(), variants.clone());
                }
                _ => {}
            }
        }
        for stmt in program.body {
            match stmt {
                Stmt::FunctionDeclaration(_) => {}
                Stmt::ClassDeclaration(_)    => {}
                Stmt::EnumDeclaration { .. } => {}
                other => {
                    self.exec_stmt(other).map_err(|e| match e {
                        ExecError::Error(s) => s,
                        ExecError::Signal(s) => format!("unexpected signal at top level: {:?}", s),
                    })?;
                }
            }
        }
        self.eval_function_call("main".to_string(), vec![])
            .map(|_| ())
    }

    pub fn exec_stmt(&mut self, stmt: Stmt) -> Result<Option<Value>, ExecError> {
        match stmt {
            Stmt::ReturnStatement(expr) => {
                let val = match expr {
                    Some(e) => self.eval_expr(e).map_err(ExecError::Error)?,
                    None => Value::Null,
                };
                Err(ExecError::Signal(Signal::Return(val)))
            }
            Stmt::BreakStatement => {
                Err(ExecError::Signal(Signal::Break))
            }
            Stmt::ContinueStatement => {
                Err(ExecError::Signal(Signal::Continue))
            }
            Stmt::ExpressionStatement(expr) => {
                self.eval_expr(expr).map_err(ExecError::Error)?;
                Ok(None)
            }
            Stmt::VariableDeclaration { name, ty, value, .. } => {
                let val = match value {
                    Some(e) => self.eval_expr(e)?,
                    None => Value::Null,
                };
                // coerce list elements to declared type
                let val = match (&ty, val) {
                    (Some(Type::List(Some(types))), Value::List(items)) => {
                        let coerced: Result<Vec<Value>, String> = items.into_iter().map(|item| {
                            self.coerce_to_type(item, &types[0])
                        }).collect();
                        Value::List(coerced?)
                    }
                    (Some(t), v) => self.coerce_to_type(v, t)?,
                    (None, v) => v,
                };
                self.env.set(&name, val);
                Ok(None)
            }
            Stmt::FunctionDeclaration(f) => {
                self.env.set(&f.name, Value::Function(f.clone()));
                Ok(None)
            }
            Stmt::ClassDeclaration(c) => {
                self.classes.insert(c.name.clone(), c);
                Ok(None)
            }
            Stmt::EnumDeclaration { name, variants } => {
                self.enums.insert(name, variants);
                Ok(None)
            }
            Stmt::ImportStatement { path, alias, items } => {
                #[cfg(feature = "stdl")]
                {
                    // check if it's a stdl import
                    if path.first().map(|s| s.as_str()) == Some("stdl") {
                        match items {
                            Some(items) => {
                                for (name, alias) in items {
                                    let key = alias.unwrap_or_else(|| name.clone());
                                    if let Some(val) = crate::stdl::get_value(&path, &name) {
                                        self.env.set(&key, val);
                                    } else if crate::stdl::is_type(&path, &name) {
                                        self.registered_types.insert(key.clone());
                                        // also register as callable so `window()` works as constructor
                                        self.imports.insert(key.clone(), path.clone());
                                        if let Some(variants) = crate::stdl::get_enum_variants(&path, &name) {
                                            self.enums.insert(key, variants);
                                        }
                                    } else {
                                        self.imports.insert(key, path.clone());
                                    }
                                }
                            }
                            None => {
                                if let Some(alias) = alias {
                                    self.imports.insert(alias, path.clone());
                                }
                            }
                        }
                        return Ok(None);
                    }
                }

                // non-stdl import -- load the file
                let file_name = format!("{}.cig", path.join("/"));
                let package_name = path.first().cloned().unwrap_or_default();

                // check relative to current file's directory first, then cwd, then local deps folder
                let file_path = {
                    // 1. relative to source file
                    let relative_to_base = format!("{}/{}", self.base_dir, file_name);
                    // 2. relative to cwd
                    let relative_to_cwd = file_name.clone();
                    // 3. local deps folder
                    let local_dep = if path.len() == 1 {
                        format!("deps/{}/src/main.cig", package_name)
                    } else {
                        format!("deps/{}/src/{}.cig", package_name, path[1..].join("/"))
                    };
                    // 4. global packages
                    let home = std::env::var("USERPROFILE")
                        .or_else(|_| std::env::var("HOME"))
                        .unwrap_or_else(|_| ".".to_string());
                    let global_dep = if path.len() == 1 {
                        format!("{}/.cigale/packages/{}/src/main.cig", home, package_name)
                    } else {
                        format!("{}/.cigale/packages/{}/src/{}.cig", home, package_name, path[1..].join("/"))
                    };

                    if std::path::Path::new(&relative_to_base).exists() {
                        relative_to_base
                    } else if std::path::Path::new(&relative_to_cwd).exists() {
                        relative_to_cwd
                    } else if std::path::Path::new(&local_dep).exists() {
                        local_dep
                    } else if std::path::Path::new(&global_dep).exists() {
                        global_dep
                    } else {
                        return Err(ExecError::Error(format!(
                            "cannot find import '{}' — searched:\n  {}\n  {}\n  {}\n  {}\n  cwd: {}\n  hint: run 'cigale fetch' to install dependencies",
                            file_name,
                            relative_to_base,
                            relative_to_cwd,
                            local_dep,
                            global_dep,
                            std::env::current_dir().unwrap_or_default().display()
                        )));
                    }
                };

                let source = match std::fs::read_to_string(&file_path) {
                    Ok(s) => s,
                    Err(e) => return Err(ExecError::Error(format!("failed to load import '{}': {}", file_path, e))),
                };
                
                // lex and parse
                let mut lexer = crate::lexer::Lexer::new(&source);
                let tokens = lexer.tokenize();
                let mut parser = crate::parser::Parser::new(tokens);
                let program = match parser.parse() {
                    Ok(p) => p,
                    Err(e) => return Err(ExecError::Error(format!("parse error in '{}': {}", file_path, e))),
                };

                // first pass -- register everything from the imported file
                for stmt in &program.body {
                    match stmt {
                        crate::parser::ast::Stmt::FunctionDeclaration(f) => {
                            if f.name != "main" {  // never register main from imports
                                self.env.set(&f.name, Value::Function(f.clone()));
                            }
                        }
                        crate::parser::ast::Stmt::ClassDeclaration(c) => {
                            self.classes.insert(c.name.clone(), c.clone());
                        }
                        crate::parser::ast::Stmt::EnumDeclaration { name, variants } => {
                            self.enums.insert(name.clone(), variants.clone());
                        }
                        _ => {}
                    }
                }

                // second pass -- execute top level non-function statements
                for stmt in program.body {
                    match stmt {
                        crate::parser::ast::Stmt::FunctionDeclaration(_) => {}
                        crate::parser::ast::Stmt::ClassDeclaration(_)    => {}
                        crate::parser::ast::Stmt::EnumDeclaration { .. } => {}
                        other => { self.exec_stmt(other)?; }
                    }
                }

                // if selective import, only expose requested names
                if let Some(items) = items {
                    // everything is already registered globally for now
                    // TODO: scope imports properly
                    let _ = items;
                }

                Ok(None)
            }
            Stmt::StaticBlock(stmts) => {
                for stmt in stmts {
                    self.exec_stmt(stmt)?;
                }
                Ok(None)
            }
            Stmt::IfStatement { condition, body, else_ifs, else_body } => {
                let cond = self.eval_expr(condition).map_err(ExecError::Error)?;
                if self.is_truthy(&cond) {
                    self.exec_block(body)?;
                } else {
                    let mut matched = false;
                    for (cond, body) in else_ifs {
                        let c = self.eval_expr(cond).map_err(ExecError::Error)?;
                        if self.is_truthy(&c) {
                            self.exec_block(body)?;
                            matched = true;
                            break;
                        }
                    }
                    if !matched {
                        if let Some(body) = else_body {
                            self.exec_block(body)?;
                        }
                    }
                }
                Ok(None)
            }
            Stmt::WhileStatement { condition, body } => {
                loop {
                    let cond = self.eval_expr(condition.clone()).map_err(ExecError::Error)?;
                    if !self.is_truthy(&cond) { break; }
                    match self.exec_block(body.clone()) {
                        Err(ExecError::Signal(Signal::Break))    => break,
                        Err(ExecError::Signal(Signal::Continue)) => continue,
                        Err(e) => return Err(e),
                        Ok(_)  => {}
                    }
                }
                Ok(None)
            }
            Stmt::ForStatement { init, condition, step, body } => {
                let old_env = std::mem::replace(&mut self.env, Environment::new());
                self.env = Environment::new_child(old_env);

                self.exec_stmt(*init)?;
                loop {
                    let cond = self.eval_expr(condition.clone()).map_err(ExecError::Error)?;
                    if !self.is_truthy(&cond) { break; }
                    match self.exec_block(body.clone()) {
                        Err(ExecError::Signal(Signal::Break))    => break,
                        Err(ExecError::Signal(Signal::Continue)) => {}
                        Err(e) => {
                            let child = std::mem::replace(&mut self.env, Environment::new());
                            self.env = *child.parent.unwrap();
                            return Err(e);
                        }
                        Ok(_) => {}
                    }
                    self.exec_stmt(*step.clone())?;
                }

                let child = std::mem::replace(&mut self.env, Environment::new());
                self.env = *child.parent.unwrap();
                Ok(None)
            }
            Stmt::ForeachStatement { ty, name, iterable, body } => {
                let list = self.eval_expr(iterable).map_err(ExecError::Error)?;
                let items = match list {
                    Value::List(items) => items,
                    _ => return Err(ExecError::Error("foreach requires a list".to_string())),
                };
                for item in items {
                    if let Some(ref t) = ty {
                        if !self.type_matches(t, &item) {
                            return Err(ExecError::Error(format!(
                                "foreach type mismatch: expected {} but got {} (value: {:?})",
                                type_to_str(t), value_to_str(&item), item
                            )));
                        }
                    }
                    let old_env = std::mem::replace(&mut self.env, Environment::new());
                    self.env = Environment::new_child(old_env);
                    self.env.set(&name, item);
                    match self.exec_block(body.clone()) {
                        Err(ExecError::Signal(Signal::Break)) => {
                            let child = std::mem::replace(&mut self.env, Environment::new());
                            self.env = *child.parent.unwrap();
                            break;
                        }
                        Err(ExecError::Signal(Signal::Continue)) => {}
                        Err(e) => {
                            let child = std::mem::replace(&mut self.env, Environment::new());
                            self.env = *child.parent.unwrap();
                            return Err(e);
                        }
                        Ok(_) => {}
                    }
                    let child = std::mem::replace(&mut self.env, Environment::new());
                    self.env = *child.parent.unwrap();
                }
                Ok(None)
            }
            Stmt::MatchStatement { value, arms } => {
                let val = self.eval_expr(value).map_err(ExecError::Error)?;
                for arm in arms {
                    if arm.is_default {
                        self.exec_block(arm.body)?;
                        break;
                    }
                    for pattern in &arm.patterns {
                        let pval = self.eval_expr(pattern.clone()).map_err(ExecError::Error)?;
                        if self.values_equal(&val, &pval) {
                            self.exec_block(arm.body.clone())?;
                            return Ok(None);
                        }
                    }
                }
                Ok(None)
            }
        }
    }

    pub fn exec_block(&mut self, stmts: Vec<Stmt>) -> Result<Option<Value>, ExecError> {
        let old_env = std::mem::replace(&mut self.env, Environment::new());
        self.env = Environment::new_child(old_env);

        let mut result = Ok(None);
        for stmt in stmts {
            result = self.exec_stmt(stmt);
            if result.is_err() { break; }
        }

        let child = std::mem::replace(&mut self.env, Environment::new());
        self.env = *child.parent.unwrap();
        result
    }

    pub fn is_truthy(&self, val: &Value) -> bool {
        match val {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::Int(n) => *n != 0,
            Value::Str(s) => !s.is_empty(),
            _ => true,
        }
    }

    pub fn type_matches(&self, ty: &Type, val: &Value) -> bool {
        match (ty, val) {
            (Type::Int, Value::Int(_))       => true,
            (Type::Float, Value::Float(_))   => true,
            (Type::Double, Value::Double(_)) => true,
            (Type::Str, Value::Str(_))       => true,
            (Type::Bool, Value::Bool(_))     => true,
            (Type::List(_), Value::List(_))  => true,
            (Type::UserType(n), Value::Instance { class_name, .. }) => n == class_name,
            (Type::UserType(n), Value::EnumVariant(enum_name, _))   => n == enum_name,
            _ => false,
        }
    }

    pub fn values_equal(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Int(x), Value::Int(y))         => x == y,
            (Value::Float(x), Value::Float(y))     => x == y,
            (Value::Double(x), Value::Double(y))   => x == y,
            (Value::Str(x), Value::Str(y))         => x == y,
            (Value::Bool(x), Value::Bool(y))       => x == y,
            (Value::Null, Value::Null)             => true,
            (Value::EnumVariant(e1, v1), Value::EnumVariant(e2, v2)) => e1 == e2 && v1 == v2,
            _ => false,
        }
    }

   pub fn eval_expr(&mut self, expr: Expr) -> Result<Value, String> {
        match expr {
            Expr::Literal(lit) => Ok(match lit {
                Literal::Int(n)    => Value::Int(n),
                Literal::Float(f)  => Value::Float(f),
                Literal::Double(d) => Value::Double(d),
                Literal::Str(s)    => Value::Str(s),
                Literal::Bool(b)   => Value::Bool(b),
                Literal::Null      => Value::Null,
            }),

            Expr::RefExpr(expr) => {
                match *expr {
                    Expr::Identifier(name) => {
                        // get current value
                        let val = self.env.get(&name)
                            .ok_or_else(|| format!("undefined variable: {}", name))?
                            .clone();
                        // create shared Rc
                        let rc = Rc::new(RefCell::new(val));
                        // register so future assignments to 'a' update the Rc
                        self.refs.insert(name, rc.clone());
                        Ok(Value::Ref(rc))
                    }
                    _ => Err("& operator requires a variable".to_string()),
                }
            }

            Expr::DerefExpr(expr) => {
                let val = self.eval_expr(*expr)?;
                match val {
                    Value::Ref(rc) => Ok(rc.borrow().clone()),
                    _ => Err("* operator requires a ref".to_string()),
                }
            }

            Expr::Identifier(name) => {
                // check env first
                if let Some(v) = self.env.get(&name) {
                    return Ok(v.clone());
                }
                // check if it's an enum type name e.g. perm, colour
                if self.enums.contains_key(&name) {
                    return Ok(Value::Identifier(name));
                }
                Err(format!("undefined variable: {}", name))
            }

            Expr::ListLiteral(items) => {
                let mut vals = Vec::new();
                for item in items {
                    vals.push(self.eval_expr(item)?);
                }
                Ok(Value::List(vals))
            }

            Expr::NullishExpr { left, right } => {
                let l = self.eval_expr(*left)?;
                if !matches!(l, Value::Null) {
                    Ok(l)
                } else {
                    self.eval_expr(*right)
                }
            }

            Expr::TypeofExpr { ty, expr } => {
                let val = self.eval_expr(*expr)?;
                Ok(Value::Bool(self.type_matches(&ty, &val)))
            }

            Expr::CastExpr { expr, to } => {
                let val = self.eval_expr(*expr)?;
                self.cast_value(val, &to)
            }

            Expr::IndexExpr { object, index } => {
                let obj = self.eval_expr(*object)?;
                let idx = self.eval_expr(*index)?;
                match (obj, idx) {
                    (Value::List(items), Value::Int(i)) => {
                        items.get(i as usize)
                            .cloned()
                            .ok_or_else(|| format!("index {} out of bounds", i))
                    }
                    _ => Err("index operator requires a list and an int index".to_string()),
                }
            }

            Expr::InterpolatedString(parts) => {
                let mut result = String::new();
                for part in parts {
                    match part {
                        StringPart::Literal(s) => result.push_str(&s),
                        StringPart::Expr(e) => {
                            let val = self.eval_expr(e)?;
                            result.push_str(&self.value_to_str(&val));
                        }
                    }
                }
                Ok(Value::Str(result))
            }

            Expr::UnaryExpr { op, expr } => {
                let val = self.eval_expr(*expr)?;
                match op {
                    UnaryOp::Not => match val {
                        Value::Bool(b) => Ok(Value::Bool(!b)),
                        _ => Err("! operator requires a bool".to_string()),
                    },
                    UnaryOp::Neg => match val {
                        Value::Int(n)    => Ok(Value::Int(-n)),
                        Value::Float(f)  => Ok(Value::Float(-f)),
                        Value::Double(d) => Ok(Value::Double(-d)),
                        _ => Err("unary - requires a number".to_string()),
                    },
                }
            }

            Expr::BinaryExpr { left, op, right } => {
                self.eval_binary(*left, op, *right)
            }
            Expr::FunctionCall { name, args } => {
                self.eval_function_call(name, args)
            }
            Expr::MethodCall { object, method, args } => {
                self.eval_method_call(*object, method, args)
            }
        }
    }

   pub fn value_to_str(&self, val: &Value) -> String {
        match val {
            Value::Int(n)    => n.to_string(),
            Value::Float(f)  => f.to_string(),
            Value::Double(d) => d.to_string(),
            Value::Str(s)    => s.clone(),
            Value::Ref(r)=> format!("ref({})", self.value_to_str(&r.borrow())),
            Value::Bool(b)   => b.to_string(),
            Value::Null      => "null".to_string(),
            Value::EnumVariant(e, v) => format!("{}.{}", e, v),
            Value::List(items) => {
                let parts: Vec<String> = items.iter().map(|v| self.value_to_str(v)).collect();
                format!("[{}]", parts.join(", "))
            }
            Value::Instance { class_name, .. } => format!("<{} instance>", class_name),
            Value::ResultVal { val, err } => match val.as_ref() {
                Value::Null => format!("err({})", self.value_to_str(err)),
                v => format!("ok({})", self.value_to_str(v)),
            },
            Value::Function(f) => format!("<func {}>", f.name),
            Value::Identifier(n) => n.clone(),
            Value::NativeHandle(handle) => {
                #[cfg(feature = "stdl")]
                {
                    match handle {
                        NativeHandle::File(h) => {
                            match h.lock() {
                                Ok(h) => {
                                    if h.closed {
                                        format!("<file {} [closed]>", h.path)
                                    } else {
                                        format!("<file {} [{}]>", h.path, h.perm)
                                    }
                                }
                                Err(_) => "<file [locked]>".to_string(),
                            }
                        }
                    }
                }
                #[cfg(not(feature = "stdl"))]
                {
                    "<native handle — enable stdl feature to use>".to_string()
                }
            }
        }
    }

   pub fn cast_value(&self, val: Value, to: &Type) -> Result<Value, String> {
        match (val, to) {
            (Value::Int(n),    Type::Float)  => Ok(Value::Float(n as f32)),
            (Value::Int(n),    Type::Double) => Ok(Value::Double(n as f64)),
            (Value::Int(n),    Type::Str)    => Ok(Value::Str(n.to_string())),
            (Value::Float(f),  Type::Int)    => {
                eprintln!("warning: float to int cast, auto-rounding");
                Ok(Value::Int(f.round() as i64))
            }
            (Value::Float(f),  Type::Double) => Ok(Value::Double(f as f64)),
            (Value::Float(f),  Type::Str)    => Ok(Value::Str(f.to_string())),
            (Value::Double(d), Type::Int)    => {
                eprintln!("warning: double to int cast, auto-rounding");
                Ok(Value::Int(d.round() as i64))
            }
            (Value::Double(d), Type::Float)  => {
                eprintln!("warning: double to float cast, precision loss");
                Ok(Value::Float(d as f32))
            }
            (Value::Double(d), Type::Str)    => Ok(Value::Str(d.to_string())),
            (Value::Str(s),    Type::Int)    => s.parse::<i64>()
                .map(Value::Int)
                .map_err(|_| format!("cannot cast \"{}\" to int", s)),
            (Value::Str(s),    Type::Float)  => s.parse::<f32>()
                .map(Value::Float)
                .map_err(|_| format!("cannot cast \"{}\" to float", s)),
            (v, Type::List(_)) => {
                // convert v to a list
                match v {
                    Value::List(_) => Ok(v), // already a list
                    Value::Str(s) => {
                        // convert string to list of chars
                        Ok(Value::List(
                            s.chars().map(|c| Value::Str(c.to_string())).collect()
                        ))
                    }
                    _ => Ok(Value::List(vec![v])),
                }
            }
            (Value::Bool(b), Type::Str) => {if b {Ok(Value::Str("true".to_string()))} else {Ok(Value::Str("false".to_string()))}}
            
            (v, t) => {
                if self.type_matches(t, &v) {
                    Ok(v)
                } else {
                    Err(format!("cannot cast {:?} to {:?}", v, t))
                }
            }
        }
    }

    pub fn eval_binary(&mut self, left: Expr, op: BinaryOp, right: Expr) -> Result<Value, String> {
        // handle assignment ops first before evaluating left
        match op {
            BinaryOp::Assign => {
                let val = self.eval_expr(right)?;
                match left {
                    Expr::Identifier(name) => {
                        // update ref registry if one exists for this variable
                        if let Some(rc) = self.refs.get(&name).cloned() {
                            *rc.borrow_mut() = val.clone();
                        }
                        if !self.env.assign(&name, val.clone()) {
                            return Err(format!("undefined variable: {}", name));
                        }
                        return Ok(val);
                    }
                    Expr::DerefExpr(inner) => {
                        match *inner {
                            Expr::Identifier(name) => {
                                let ref_val = self.env.get(&name)
                                    .ok_or_else(|| format!("undefined variable: {}", name))?
                                    .clone();
                                match ref_val {
                                    Value::Ref(rc) => {
                                        *rc.borrow_mut() = val.clone();
                                        // update original variable in env
                                        for (var_name, var_rc) in &self.refs {
                                            if Rc::ptr_eq(var_rc, &rc) {
                                                self.env.assign(var_name, val.clone());
                                                break;
                                            }
                                        }
                                        return Ok(val);
                                    }
                                    _ => return Err(format!("{} is not a ref", name)),
                                }
                            }
                            _ => return Err("cannot deref non-identifier".to_string()),
                        }
                    }
                    Expr::IndexExpr { object, index } => {
                        let idx = self.eval_expr(*index)?;
                        match *object {
                            Expr::Identifier(name) => {
                                let list = self.env.get(&name)
                                    .ok_or_else(|| format!("undefined variable: {}", name))?
                                    .clone();
                                match (list, idx) {
                                    (Value::List(mut items), Value::Int(i)) => {
                                        items[i as usize] = val;
                                        self.env.assign(&name, Value::List(items));
                                        return Ok(Value::Null);
                                    }
                                    _ => return Err("index assign requires a list".to_string()),
                                }
                            }
                            _ => return Err("cannot assign to index of non-identifier".to_string()),
                        }
                    }
                    _ => return Err("invalid assignment target".to_string()),
                }
            }
            // compound assignment ops -- desugar to: left = left op right
            BinaryOp::AddAssign | BinaryOp::SubAssign | BinaryOp::MulAssign |
            BinaryOp::DivAssign | BinaryOp::ModuloAssign | BinaryOp::PercentAssign |
            BinaryOp::ExpAssign | BinaryOp::XORAssign | BinaryOp::ANDAssign |
            BinaryOp::ORAssign  | BinaryOp::BitShiftLeftAssign | BinaryOp::BitShiftRightAssign => {
                let base_op = match op {
                    BinaryOp::AddAssign          => BinaryOp::Add,
                    BinaryOp::SubAssign          => BinaryOp::Sub,
                    BinaryOp::MulAssign          => BinaryOp::Mul,
                    BinaryOp::DivAssign          => BinaryOp::Div,
                    BinaryOp::ModuloAssign       => BinaryOp::Modulo,
                    BinaryOp::PercentAssign      => BinaryOp::Percent,
                    BinaryOp::ExpAssign          => BinaryOp::Exp,
                    BinaryOp::XORAssign          => BinaryOp::XOR,
                    BinaryOp::ANDAssign          => BinaryOp::AND,
                    BinaryOp::ORAssign           => BinaryOp::OR,
                    BinaryOp::BitShiftLeftAssign => BinaryOp::BitShiftLeft,
                    BinaryOp::BitShiftRightAssign=> BinaryOp::BitShiftRight,
                    _ => unreachable!(),
                };
                let val = self.eval_binary(left.clone(), base_op, right)?;
                match left {
                    Expr::Identifier(name) => {
                        if !self.env.assign(&name, val.clone()) {
                            return Err(format!("undefined variable: {}", name));
                        }
                        return Ok(val);
                    }
                    _ => return Err("invalid compound assignment target".to_string()),
                }
            }
            _ => {}
        }

        // evaluate both sides for non-assignment ops
        let lval = self.eval_expr(left)?;
        let rval = self.eval_expr(right)?;

        match op {
            // arithmetic
            BinaryOp::Add => match (lval, rval) {
                (Value::Int(a),    Value::Int(b))    => Ok(Value::Int(a + b)),
                (Value::Float(a),  Value::Float(b))  => Ok(Value::Float(a + b)),
                (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a + b)),
                (Value::Float(a),  Value::Int(b))    => Ok(Value::Float(a + b as f32)),
                (Value::Int(a),    Value::Float(b))  => Ok(Value::Float(a as f32 + b)),
                (Value::Double(a), Value::Int(b))    => Ok(Value::Double(a + b as f64)),
                (Value::Int(a),    Value::Double(b)) => Ok(Value::Double(a as f64 + b)),
                (Value::Str(a),    Value::Str(b))    => Ok(Value::Str(a + &b)),
                (a, b) => Err(format!("cannot add {:?} and {:?}", a, b)),
            },
            BinaryOp::Sub => match (lval, rval) {
                (Value::Int(a),    Value::Int(b))    => Ok(Value::Int(a - b)),
                (Value::Float(a),  Value::Float(b))  => Ok(Value::Float(a - b)),
                (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a - b)),
                (Value::Float(a),  Value::Int(b))    => Ok(Value::Float(a - b as f32)),
                (Value::Int(a),    Value::Float(b))  => Ok(Value::Float(a as f32 - b)),
                (Value::Double(a), Value::Int(b))    => Ok(Value::Double(a - b as f64)),
                (Value::Int(a),    Value::Double(b)) => Ok(Value::Double(a as f64 - b)),
                (a, b) => Err(format!("cannot subtract {:?} and {:?}", a, b)),
            },
            BinaryOp::Mul => match (lval, rval) {
                (Value::Int(a),    Value::Int(b))    => Ok(Value::Int(a * b)),
                (Value::Float(a),  Value::Float(b))  => Ok(Value::Float(a * b)),
                (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a * b)),
                (Value::Float(a),  Value::Int(b))    => Ok(Value::Float(a * b as f32)),
                (Value::Int(a),    Value::Float(b))  => Ok(Value::Float(a as f32 * b)),
                (Value::Double(a), Value::Int(b))    => Ok(Value::Double(a * b as f64)),
                (Value::Int(a),    Value::Double(b)) => Ok(Value::Double(a as f64 * b)),
                (a, b) => Err(format!("cannot multiply {:?} and {:?}", a, b)),
            },
            BinaryOp::Div => match (lval, rval) {
                (Value::Int(a),    Value::Int(b))    => Ok(Value::Float(a as f32 / b as f32)),
                (Value::Float(a),  Value::Float(b))  => Ok(Value::Float(a / b)),
                (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a / b)),
                (Value::Float(a),  Value::Int(b))    => Ok(Value::Float(a / b as f32)),
                (Value::Int(a),    Value::Float(b))  => Ok(Value::Float(a as f32 / b)),
                (Value::Double(a), Value::Int(b))    => Ok(Value::Double(a / b as f64)),
                (Value::Int(a),    Value::Double(b)) => Ok(Value::Double(a as f64 / b)),
                (a, b) => Err(format!("cannot divide {:?} and {:?}", a, b)),
            },
            BinaryOp::Modulo => match (lval, rval) {
                (Value::Int(a),   Value::Int(b))   => Ok(Value::Int(a % b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a % b)),
                (Value::Float(a), Value::Int(b))   => Ok(Value::Float(a % b as f32)),
                (Value::Int(a),   Value::Float(b)) => Ok(Value::Float(a as f32 % b)),
                (a, b) => Err(format!("cannot modulo {:?} and {:?}", a, b)),
            },
            BinaryOp::Percent => match (lval, rval) {
                (Value::Int(a),    Value::Int(b))    => Ok(Value::Float(a as f32 * b as f32 / 100.0)),
                (Value::Float(a),  Value::Float(b))  => Ok(Value::Float(a * b / 100.0)),
                (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a * b / 100.0)),
                (Value::Float(a),  Value::Int(b))    => Ok(Value::Float(a * b as f32 / 100.0)),
                (Value::Int(a),    Value::Float(b))  => Ok(Value::Float(a as f32 * b / 100.0)),
                (a, b) => Err(format!("cannot percent {:?} and {:?}", a, b)),
            },
            BinaryOp::Exp => match (lval, rval) {
                (Value::Int(a),    Value::Int(b))    => Ok(Value::Int(a.pow(b as u32))),
                (Value::Float(a),  Value::Float(b))  => Ok(Value::Float(a.powf(b))),
                (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a.powf(b))),
                (Value::Float(a),  Value::Int(b))    => Ok(Value::Float(a.powf(b as f32))),
                (Value::Int(a),    Value::Float(b))  => Ok(Value::Float((a as f32).powf(b))),
                (Value::Double(a), Value::Int(b))    => Ok(Value::Double(a.powf(b as f64))),
                (Value::Int(a),    Value::Double(b)) => Ok(Value::Double((a as f64).powf(b))),
                (a, b) => Err(format!("cannot exponentiate {:?} and {:?}", a, b)),
            },
            // comparison
            BinaryOp::Equal    => Ok(Value::Bool(self.values_equal(&lval, &rval))),
            BinaryOp::NotEqual => Ok(Value::Bool(!self.values_equal(&lval, &rval))),
            BinaryOp::Less => match (lval, rval) {
                (Value::Int(a),    Value::Int(b))    => Ok(Value::Bool(a < b)),
                (Value::Float(a),  Value::Float(b))  => Ok(Value::Bool(a < b)),
                (Value::Double(a), Value::Double(b)) => Ok(Value::Bool(a < b)),
                (Value::Float(a),  Value::Int(b))    => Ok(Value::Bool(a < b as f32)),
                (Value::Int(a),    Value::Float(b))  => Ok(Value::Bool((a as f32) < b)),
                (Value::Double(a), Value::Int(b))    => Ok(Value::Bool(a < b as f64)),
                (Value::Int(a),    Value::Double(b)) => Ok(Value::Bool((a as f64) < b)),
                (a, b) => Err(format!("cannot compare {:?} and {:?}", a, b)),
            },
            BinaryOp::Greater => match (lval, rval) {
                (Value::Int(a),    Value::Int(b))    => Ok(Value::Bool(a > b)),
                (Value::Float(a),  Value::Float(b))  => Ok(Value::Bool(a > b)),
                (Value::Double(a), Value::Double(b)) => Ok(Value::Bool(a > b)),
                (Value::Float(a),  Value::Int(b))    => Ok(Value::Bool(a > b as f32)),
                (Value::Int(a),    Value::Float(b))  => Ok(Value::Bool((a as f32) > b)),
                (Value::Double(a), Value::Int(b))    => Ok(Value::Bool(a > b as f64)),
                (Value::Int(a),    Value::Double(b)) => Ok(Value::Bool((a as f64) > b)),
                (a, b) => Err(format!("cannot compare {:?} and {:?}", a, b)),
            },
            BinaryOp::LessEqual => match (lval, rval) {
                (Value::Int(a),    Value::Int(b))    => Ok(Value::Bool(a <= b)),
                (Value::Float(a),  Value::Float(b))  => Ok(Value::Bool(a <= b)),
                (Value::Double(a), Value::Double(b)) => Ok(Value::Bool(a <= b)),
                (Value::Float(a),  Value::Int(b))    => Ok(Value::Bool(a <= b as f32)),
                (Value::Int(a),    Value::Float(b))  => Ok(Value::Bool((a as f32) <= b)),
                (Value::Double(a), Value::Int(b))    => Ok(Value::Bool(a <= b as f64)),
                (Value::Int(a),    Value::Double(b)) => Ok(Value::Bool((a as f64) <= b)),
                (a, b) => Err(format!("cannot compare {:?} and {:?}", a, b)),
            },
            BinaryOp::GreaterEqual => match (lval, rval) {
                (Value::Int(a),    Value::Int(b))    => Ok(Value::Bool(a >= b)),
                (Value::Float(a),  Value::Float(b))  => Ok(Value::Bool(a >= b)),
                (Value::Double(a), Value::Double(b)) => Ok(Value::Bool(a >= b)),
                (Value::Float(a),  Value::Int(b))    => Ok(Value::Bool(a >= b as f32)),
                (Value::Int(a),    Value::Float(b))  => Ok(Value::Bool((a as f32) >= b)),
                (Value::Double(a), Value::Int(b))    => Ok(Value::Bool(a >= b as f64)),
                (Value::Int(a),    Value::Double(b)) => Ok(Value::Bool((a as f64) >= b)),
                (a, b) => Err(format!("cannot compare {:?} and {:?}", a, b)),
            },
            // logical
            BinaryOp::And => Ok(Value::Bool(self.is_truthy(&lval) && self.is_truthy(&rval))),
            BinaryOp::Or  => Ok(Value::Bool(self.is_truthy(&lval) || self.is_truthy(&rval))),
            // bitwise
            BinaryOp::XOR => match (lval, rval) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a ^ b)),
                (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a ^ b)),
                (a, b) => Err(format!("cannot XOR {:?} and {:?}", a, b)),
            },
            BinaryOp::AND => match (lval, rval) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a & b)),
                (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a & b)),
                (a, b) => Err(format!("cannot AND {:?} and {:?}", a, b)),
            },
            BinaryOp::OR => match (lval, rval) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a | b)),
                (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a | b)),
                (a, b) => Err(format!("cannot OR {:?} and {:?}", a, b)),
            },
            BinaryOp::BitShiftLeft => match (lval, rval) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a << b)),
                (a, b) => Err(format!("cannot bit shift {:?} and {:?}", a, b)),
            },
            BinaryOp::BitShiftRight => match (lval, rval) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a >> b)),
                (a, b) => Err(format!("cannot bit shift {:?} and {:?}", a, b)),
            },
            // already handled above
            _ => unreachable!(),
        }
    }

   pub fn eval_function_call(&mut self, name: String, args: Vec<Expr>) -> Result<Value, String> {
        // evaluate args first
        let mut arg_vals = Vec::new();
        for arg in args {
            arg_vals.push(self.eval_expr(arg)?);
        }

        // stdl functions
        #[cfg(feature = "stdl")]
        if let Some(module_path) = self.imports.get(&name).cloned() {
            if let Some(result) = crate::stdl::handle(&module_path, &name, arg_vals.clone()) {
                return result;
            }
        }

        // check if it's a class instantiation
        if self.classes.contains_key(&name) {
            let class = self.classes.get(&name).unwrap().clone();
            let mut fields = HashMap::new();

            // initialize parent fields first if class has a parent
            if let Some(ref parent_name) = class.parent {
                if let Some(parent_class) = self.classes.get(parent_name).cloned() {
                    // parent methods
                    for method in &parent_class.body {
                        fields.insert(method.name.clone(), Value::Function(method.clone()));
                    }
                    // parent fields
                    for field in &parent_class.fields {
                        if let Stmt::VariableDeclaration { name, value, .. } = field {
                            let val = match value {
                                Some(e) => self.eval_expr(e.clone())?,
                                None => Value::Null,
                            };
                            fields.insert(name.clone(), val);
                        }
                    }
                }
            }

            // child methods override parent
            for method in &class.body {
                fields.insert(method.name.clone(), Value::Function(method.clone()));
            }
            // child fields override parent
            for field in &class.fields {
                if let Stmt::VariableDeclaration { name, value, .. } = field {
                    let val = match value {
                        Some(e) => self.eval_expr(e.clone())?,
                        None => Value::Null,
                    };
                    fields.insert(name.clone(), val);
                }
            }

            return Ok(Value::Instance {
                class_name: name,
                parent: class.parent.clone(),
                fields,
            });
        }

        // check if it's an enum variant used as constructor
        if self.enums.contains_key(&name) {
            return Err(format!("{} is an enum, not a function", name));
        }

        // look up user defined function
        let func = match self.env.get(&name) {
            Some(Value::Function(f)) => f.clone(),
            Some(_) => return Err(format!("{} is not a function", name)),
            None => return Err(format!("undefined function: {}", name)),
        };

        self.call_function(func, arg_vals)
    }

   pub fn call_function(&mut self, func: FunctionDecl, args: Vec<Value>) -> Result<Value, String> {
        if args.len() != func.params.len() {
            return Err(format!(
                "function {} expects {} args but got {}",
                func.name, func.params.len(), args.len()
            ));
        }

        let old_env = std::mem::replace(&mut self.env, Environment::new());
        self.env = Environment::new_child(old_env);

        for (param, val) in func.params.iter().zip(args.into_iter()) {
            self.env.set(&param.name, val);
        }

        // Borrow body instead of moving it
        if func.body.is_none() {
            // This is a field. Return the value stored in the instance env
            if let Some(val) = self.env.get(&func.name) {
                return Ok(val.clone());
            } else {
                return Ok(Value::Null); // default null if not set
            }
        }

        let body = func.body.as_ref().unwrap(); // borrow for iteration

        let mut return_val = Value::Null;
        for stmt in body {
            match self.exec_stmt(stmt.clone()) {
                Ok(_) => {}
                Err(ExecError::Signal(Signal::Return(val))) => {
                    return_val = val;    // clean! no string parsing
                    break;
                }
                Err(ExecError::Error(e)) => {
                    let child = std::mem::replace(&mut self.env, Environment::new());
                    self.env = *child.parent.unwrap();
                    return Err(e);
                }
                Err(ExecError::Signal(s)) => {
                    // break/continue outside loop -- shouldn't happen but handle gracefully
                    let child = std::mem::replace(&mut self.env, Environment::new());
                    self.env = *child.parent.unwrap();
                    return Err(format!("unexpected signal {:?} in function body", s));
                }
            }
        }

        let child = std::mem::replace(&mut self.env, Environment::new());
        self.env = *child.parent.unwrap();

        Ok(return_val)
    }

    pub fn eval_method_call(&mut self, object: Expr, method: String, args: Vec<Expr>) -> Result<Value, String> {
            // evaluate args
            let mut arg_vals = Vec::new();
            for arg in args {
                arg_vals.push(self.eval_expr(arg)?);
            }
            let obj_val = self.eval_expr(object.clone())?;

            match obj_val {
                Value::ResultVal { val, err } => {
                    match method.as_str() {
                        "get" => {
                            if !matches!(*err, Value::Null) {
                                return Err("called get() on error result".to_string());
                            }
                            Ok(*val)
                        }

                        "error" => {
                            if matches!(*err, Value::Null) {
                                Ok(Value::Null)
                            } else {
                                Ok(*err)
                            }
                        }

                        "is_ok" => Ok(Value::Bool(matches!(*err, Value::Null))),
                        "is_err" => Ok(Value::Bool(!matches!(*err, Value::Null))),

                        _ => Err(format!("result has no method {}", method)),
                    }
                }
                // enum variant -- cannot call methods on it
                Value::EnumVariant(enum_name, _) => {
                    Err(format!("cannot call method {} on enum variant {}", method, enum_name))
                }
                Value::NativeHandle(handle) => {
                    #[cfg(feature = "stdl")]
                    match handle {
                        NativeHandle::File(h) => {
                            crate::stdl::io::file_method(h, &method, arg_vals)
                        }
                    }
                    #[cfg(not(feature = "stdl"))]
                    Err("native handles require stdl feature".to_string())
                }
                // Error instance field access
                Value::Instance { ref class_name, ref fields, .. } if class_name == "Error" => {
                    match method.as_str() {
                        "msg" => Ok(fields.get("msg").cloned().unwrap_or(Value::Null)),
                        _ => self.eval_instance_method(&object, obj_val, method, arg_vals),
                    }
                }
                // enum access: colour.RED
                Value::Identifier(enum_name) => {
                    match self.enums.get(&enum_name) {
                        Some(variants) if variants.contains(&method) => {
                            Ok(Value::EnumVariant(enum_name, method))
                        }
                        _ => Err(format!("no variant {} on {}", method, enum_name)),
                    }
                }
                // general instance method call
                Value::Instance { .. } => {
                    self.eval_instance_method(&object, obj_val, method, arg_vals)
                }
                Value::List(items) => {
                    match method.as_str() {
                        "len" => Ok(Value::Int(items.len() as i64)),
                        "first" => Ok(items.first().cloned().unwrap_or(Value::Null)),
                        "last"  => Ok(items.last().cloned().unwrap_or(Value::Null)),
                        "contains" => {
                            let val = arg_vals.into_iter().next().unwrap_or(Value::Null);
                            Ok(Value::Bool(items.iter().any(|v| self.values_equal(v, &val))))
                        }
                        "push" | "pop" | "remove" | "reverse" | "insert" => {
                            // these mutate -- need identifier to write back
                            match object {
                                Expr::Identifier(var_name) => {
                                    let mut list = match self.env.get(&var_name) {
                                        Some(Value::List(l)) => l.clone(),
                                        _ => return Err(format!("'{}' is not a list", var_name)),
                                    };
                                    match method.as_str() {
                                        "push" => {
                                            let val = arg_vals.into_iter().next().unwrap_or(Value::Null);
                                            list.push(val);
                                            self.env.assign(&var_name, Value::List(list));
                                            Ok(Value::Null)
                                        }
                                        "pop" => {
                                            let val = list.pop().unwrap_or(Value::Null);
                                            self.env.assign(&var_name, Value::List(list));
                                            Ok(val)
                                        }
                                        "remove" => {
                                            let idx = match arg_vals.first() {
                                                Some(Value::Int(i)) => *i as usize,
                                                _ => return Err("remove() requires an int index".to_string()),
                                            };
                                            if idx >= list.len() {
                                                return Err(format!("index {} out of bounds", idx));
                                            }
                                            let val = list.remove(idx);
                                            self.env.assign(&var_name, Value::List(list));
                                            Ok(val)
                                        }
                                        "reverse" => {
                                            list.reverse();
                                            self.env.assign(&var_name, Value::List(list));
                                            Ok(Value::Null)
                                        }
                                        "insert" => {
                                            let idx = match arg_vals.get(0) {
                                                Some(Value::Int(i)) => *i as usize,
                                                _ => return Err("insert() requires an int index".to_string()),
                                            };
                                            let val = arg_vals.into_iter().nth(1).unwrap_or(Value::Null);
                                            if idx > list.len() {
                                                return Err(format!("index {} out of bounds", idx));
                                            }
                                            list.insert(idx, val);
                                            self.env.assign(&var_name, Value::List(list));
                                            Ok(Value::Null)
                                        }
                                        _ => unreachable!()
                                    }
                                }
                                _ => Err(format!("cannot call mutating method '{}' on a non-variable list", method)),
                            }
                        }
                        _ => Err(format!("list has no method {}", method)),
                    }
                }
                Value::Str(s) => {
                    match method.as_str() {
                        "join" => {
                            let val = match arg_vals.get(0) {
                                Some(Value::Str(st)) => st,
                                _ => return Err("join() requires a string".to_string()),
                            };

                            Ok(Value::Str(s + val))
                        },
                        "index" => {
                            let val = match arg_vals.get(0) {
                                Some(Value::Int( int )) => int,
                                _ => return Err("index() requires an int index".to_string())
                            };
                            if *val as usize > s.len() - 1 {
                                return Err(format!("index {} out of bounds", val));
                            }
                            let ch = s.chars().nth(*val as usize).unwrap();
                            Ok(Value::Str(ch.to_string()))
                        },
                        "len" => Ok(Value::Int(s.chars().count() as i64)),
                        _ => Err(format!("str has no method {}", method)),
                    }
                }
                _ => Err(format!("cannot call method {} on {:?}", method, obj_val)),
            }
        }

    pub fn eval_instance_method(&mut self, object: &Expr, obj: Value, method: String, args: Vec<Value>) -> Result<Value, String> {
            match obj {
                    Value::Instance { class_name, fields, parent } => {
                        // check for field access first (no args, field exists)
                        if args.is_empty() {
                            if let Some(val) = fields.get(&method) {
                                match val.clone() {
                                    Value::Function(_) => {
                                        // it's a method stored as a function, call it with fields in scope
                                    }
                                    v => return Ok(v), // it's a plain field value
                                }
                            }
                        }

                        // look up method in class definition
                        let class = self.classes.get(&class_name)
                            .ok_or_else(|| format!("undefined class {}", class_name))?
                            .clone();
                        let func = class.body.iter()
                            .find(|f| f.name == method)
                            .ok_or_else(|| format!("class {} has no method {}", class_name, method))?
                            .clone();

                        // validate arg count
                        if args.len() != func.params.len() {
                            return Err(format!(
                                "method {} expects {} args but got {}",
                                func.name, func.params.len(), args.len()
                            ));
                        }

                        // create new scope
                        let old_env = std::mem::replace(&mut self.env, Environment::new());
                        self.env = Environment::new_child(old_env);

                        // bind fields FIRST so methods can access them
                        for (k, v) in &fields {
                            self.env.set(k, v.clone());
                        }

                        // then bind params on top
                        for (param, val) in func.params.iter().zip(args.into_iter()) {
                            self.env.set(&param.name, val);
                        }

                        // execute body
                        let body = match func.body {
                            Some(b) => b,
                            None => {
                                let child = std::mem::replace(&mut self.env, Environment::new());
                                self.env = *child.parent.unwrap();
                                return Err(format!("cannot call blank method {}", func.name));
                            }
                        };

                        let mut return_val = Value::Null;
                        for stmt in body {
                            match self.exec_stmt(stmt) {
                                Ok(_) => {}
                                Err(ExecError::Signal(Signal::Return(val))) => {
                                    return_val = val;
                                    break;
                                }
                                Err(ExecError::Error(e)) => {
                                    let child = std::mem::replace(&mut self.env, Environment::new());
                                    self.env = *child.parent.unwrap();
                                    return Err(e);
                                }
                                Err(ExecError::Signal(s)) => {
                                    let child = std::mem::replace(&mut self.env, Environment::new());
                                    self.env = *child.parent.unwrap();
                                    return Err(format!("unexpected signal {:?} in method body", s));
                                }
                            }
                        }

                        let mut updated_fields = fields.clone();
                        for key in updated_fields.keys().cloned().collect::<Vec<_>>() {
                            if let Some(new_val) = self.env.get(&key) {
                                updated_fields.insert(key, new_val.clone());
                            }
                        }

                        if let Expr::Identifier(var_name) = object {
                            self.env.assign(var_name, Value::Instance {
                                class_name: class_name.clone(),
                                fields: updated_fields,
                                parent,
                            });
                        }

                        // restore env
                        let child = std::mem::replace(&mut self.env, Environment::new());
                        self.env = *child.parent.unwrap();

                        Ok(return_val)
                    }
                    _ => Err(format!("cannot call method {} on non-instance", method)),
                }
            }

        fn coerce_to_type(&self, val: Value, ty: &Type) -> Result<Value, String> {
            match (val, ty) {
                (Value::Int(n),   Type::Float)  => Ok(Value::Float(n as f32)),
                (Value::Int(n),   Type::Double) => Ok(Value::Double(n as f64)),
                (Value::Float(f), Type::Double) => Ok(Value::Double(f as f64)),
                (v, _) => Ok(v), // no coercion needed
            }
        }
}


fn value_to_str(ty: &Value) -> String {
    match ty {
        Value::Int(..) => "int".to_string(),
        Value::Float(..) => "float".to_string(),
        Value::Double(..) => "double".to_string(),
        Value::Str(..) => "str".to_string(),
        Value::Bool(..) => "bool".to_string(),
        Value::List(values) => {
            let inner = values.iter().map(value_to_str).collect::<Vec<_>>().join(" | ");
            format!("list<{}>", inner)
        },
        Value::EnumVariant(..) => "enum".to_string(),
        Value::Instance { .. } => "class".to_string(),
        Value::ResultVal { .. } => "result".to_string(),
        Value::NativeHandle( .. ) => "stdl_native".to_string(),
        Value::Function( .. ) => "function".to_string(),
        Value::Identifier( .. ) => "identifier".to_string(),
        Value::Null => "null".to_string(),
        Value::Ref( .. ) => "ref".to_string(),
    }
}