// analyser/mod.rs
use crate::parser::ast::*;
use std::{collections::HashMap};

#[derive(Debug, Clone)]
pub enum AnalysisError {
    Error(String),
    Warning(String),
}

pub struct Analyser {
    // variable name -> type
    vars: HashMap<String, Type>,
    // function name -> (param_types, return_type)
    functions: HashMap<String, (Vec<Type>, Option<Type>)>,
    // class name -> ClassDecl
    classes: HashMap<String, ClassDecl>,
    // enum name -> variants
    enums: HashMap<String, Vec<String>>,
    errors: Vec<AnalysisError>,
    current_class: Option<String>,
}

impl Analyser {
    pub fn new() -> Self {
        Analyser {
            vars: HashMap::new(),
            functions: HashMap::new(),
            classes: HashMap::new(),
            enums: HashMap::new(),
            errors: Vec::new(),
            current_class: None,
        }
    }

    pub fn analyse(&mut self, program: &Program) -> Vec<AnalysisError> {
        // first pass -- register functions, classes, enums
        for stmt in &program.body {
            match stmt {
                Stmt::FunctionDeclaration(f) => {
                    self.functions.insert(
                        f.name.clone(),
                        (f.param_types.clone(), f.return_type.clone())
                    );
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
        // second pass -- analyse
        for stmt in &program.body {
            self.analyse_stmt(stmt);
        }
        self.errors.clone()
    }

    fn analyse_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VariableDeclaration { name, ty, value, .. } => {
                if let (Some(ty), Some(expr)) = (ty, value) {
                    if let Some(expr_ty) = self.infer_type(expr) {
                        if !self.types_compatible(ty, &expr_ty) {
                            self.errors.push(AnalysisError::Error(format!(
                                "type mismatch: variable '{}' declared as {} but assigned {}",
                                name, type_to_str(ty), type_to_str(&expr_ty)
                            )));
                        }
                    }
                }
                if let Some(ty) = ty {
                    self.vars.insert(name.clone(), ty.clone());
                }
            }
            Stmt::FunctionDeclaration(f) => {
                self.analyse_function(f);
            }
            Stmt::ClassDeclaration(c) => {
                self.analyse_class(c);
            }
            Stmt::IfStatement { condition, body, else_ifs, else_body } => {
                self.analyse_expr(condition);
                for stmt in body { self.analyse_stmt(stmt); }
                for (cond, body) in else_ifs {
                    self.analyse_expr(cond);
                    for stmt in body { self.analyse_stmt(stmt); }
                }
                if let Some(body) = else_body {
                    for stmt in body { self.analyse_stmt(stmt); }
                }
            }
            Stmt::WhileStatement { condition, body } => {
                self.analyse_expr(condition);
                for stmt in body { self.analyse_stmt(stmt); }
            }
            Stmt::ForStatement { init, condition, step, body } => {
                self.analyse_stmt(init);
                self.analyse_expr(condition);
                self.analyse_stmt(step);
                for stmt in body { self.analyse_stmt(stmt); }
            }
            Stmt::ForeachStatement { ty, name, iterable, body } => {
                self.analyse_expr(iterable);
                if let Some(ty) = ty {
                    self.vars.insert(name.clone(), ty.clone());
                }
                for stmt in body { self.analyse_stmt(stmt); }
            }
            Stmt::ReturnStatement(expr) => {
                if let Some(expr) = expr {
                    self.analyse_expr(expr);
                }
            }
            Stmt::ExpressionStatement(expr) => {
                self.analyse_expr(expr);
            }
            Stmt::StaticBlock(stmts) => {
                for stmt in stmts { self.analyse_stmt(stmt); }
            }
            Stmt::MatchStatement { value, arms } => {
                self.analyse_expr(value);
                for arm in arms {
                    for pattern in &arm.patterns { self.analyse_expr(pattern); }
                    for stmt in &arm.body { self.analyse_stmt(stmt); }
                }
            }
            _ => {}
        }
    }

    fn analyse_function(&mut self, f: &FunctionDecl) {

        // register params
        for (param, ty) in f.params.iter().zip(f.param_types.iter()) {
            self.vars.insert(param.name.clone(), ty.clone());
        }
        // analyse body
        if let Some(body) = &f.body {
            for stmt in body {
                self.analyse_stmt(stmt);
                // check return type
                if let Stmt::ReturnStatement(Some(expr)) = stmt {
                    if let Some(ret_ty) = &f.return_type {
                        if let Some(expr_ty) = self.infer_type(expr) {
                            if !self.types_compatible(ret_ty, &expr_ty) {
                                self.errors.push(AnalysisError::Error(format!(
                                    "return type mismatch in '{}': expected {:?} but got {:?}",
                                    f.name, ret_ty, expr_ty
                                )));
                            }
                        }
                    }
                }
            }
        }
    }

    fn analyse_class(&mut self, c: &ClassDecl) {
        let previous = self.current_class.clone();
        self.current_class = Some(c.name.clone());

        for method in &c.body {
            // func blank only in class inst
            if method.modifiers.is_blank && !c.is_inst {
                self.errors.push(AnalysisError::Error(format!(
                    "func blank '{}' not allowed in non-inst class '{}'",
                    method.name, c.name
                )));
            }
            // func impl warning if no matching blank in parent
            if method.modifiers.is_impl {
                if let Some(ref parent_name) = c.parent {
                    if let Some(parent) = self.classes.get(parent_name) {
                        let has_blank = parent.body.iter().any(|f| {
                            f.name == method.name && f.modifiers.is_blank
                        });
                        if !has_blank {
                            self.errors.push(AnalysisError::Warning(format!(
                                "func impl '{}' in '{}' has no matching func blank in parent '{}'",
                                method.name, c.name, parent_name
                            )));
                        }
                    }
                }
            }
        }
        // check all blank methods are implemented
        if let Some(ref parent_name) = c.parent {
            if let Some(parent) = self.classes.get(parent_name).cloned() {
                for blank in parent.body.iter().filter(|f| f.modifiers.is_blank) {
                    let implemented = c.body.iter().any(|f| {
                        f.name == blank.name && f.modifiers.is_impl
                    });
                    if !implemented {
                        self.errors.push(AnalysisError::Error(format!(
                            "class '{}' does not implement func blank '{}' from '{}'",
                            c.name, blank.name, parent_name
                        )));
                    }
                }
            }
        }
        // analyse methods
        for method in &c.body {
            self.analyse_function(method);
        }

        self.current_class = previous;
    }

    fn analyse_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::MethodCall { object, method, args } => {
                if let Some(obj_ty) = self.infer_type(object) {
                    if let Type::UserType(class_name) = &obj_ty {
                        if let Some(class) = self.classes.get(class_name).cloned() {
                            // check method access
                            if let Some(func) = class.body.iter().find(|f| f.name == *method) {
                                match func.modifiers.access {
                                    AccessModifier::Private => {
                                        // only accessible from same class
                                        if self.current_class.as_deref() != Some(class_name) {
                                            self.errors.push(AnalysisError::Error(format!(
                                                "cannot access private method '{}' of class '{}' from {}",
                                                method, class_name,
                                                self.current_class.as_deref().unwrap_or("outside its class")
                                            )));
                                        }
                                    }
                                    AccessModifier::Protected => {
                                        // only accessible from same class or subclass
                                        match &self.current_class {
                                            None => {
                                                self.errors.push(AnalysisError::Error(format!(
                                                    "cannot access protected method '{}' of class '{}' from outside of its class",
                                                    method, class_name
                                                )));
                                            }
                                            Some(caller) => {
                                                if !self.inherits_from(caller, class_name) {
                                                    self.errors.push(AnalysisError::Error(format!(
                                                        "cannot access protected method '{}' of class '{}' from '{}'",
                                                        method, class_name, caller
                                                    )));
                                                }
                                            }
                                        }
                                    }
                                    AccessModifier::Public => {} // always ok
                                }
                            }
                            // check field access
                            for field in &class.fields {
                                if let Stmt::VariableDeclaration { name, access, .. } = field {
                                    if name == method {
                                        match access {
                                            Some(AccessModifier::Private) => {
                                                if self.current_class.as_deref() != Some(class_name.as_str()) {
                                                    self.errors.push(AnalysisError::Error(format!(
                                                        "cannot access private field '{}' of class '{}' from {}",
                                                        method, class_name,
                                                        self.current_class.as_deref().unwrap_or("outside its class")
                                                    )));
                                                }
                                            }
                                            Some(AccessModifier::Protected) => {
                                                match &self.current_class {
                                                    None => {
                                                        self.errors.push(AnalysisError::Error(format!(
                                                            "cannot access protected field '{}' of class '{}' from outside its class",
                                                            method, class_name
                                                        )));
                                                    }
                                                    Some(caller) => {
                                                        if !self.inherits_from(caller, class_name) {
                                                            self.errors.push(AnalysisError::Error(format!(
                                                                "cannot access protected field '{}' of class '{}' from '{}'",
                                                                method, class_name, caller
                                                            )));
                                                        }
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                for arg in args { self.analyse_expr(arg); }
            }

            Expr::Identifier( .. ) => {}
            Expr::Literal( .. ) => {}

            Expr::FunctionCall { name, args } => {
                if let Some((param_types, _)) = self.functions.get(name).cloned() {
                    // check arg count
                    if args.len() != param_types.len() {
                        self.errors.push(AnalysisError::Error(format!(
                            "function '{}' expects {} arguments but got {}",
                            name, param_types.len(), args.len()
                        )));
                    }
                    // check arg types
                    for (arg, param_ty) in args.iter().zip(param_types.iter()) {
                        if let Some(arg_ty) = self.infer_type(arg) {
                            if !self.types_compatible(param_ty, &arg_ty) {
                                self.errors.push(AnalysisError::Error(format!(
                                    "argument type mismatch in call to '{}': expected {:?} but got {:?}",
                                    name, param_ty, arg_ty
                                )));
                            }
                        }
                    }
                }
                for arg in args { self.analyse_expr(arg); }
            }
            Expr::BinaryExpr { left, op: _, right } => {
                self.analyse_expr(left);
                self.analyse_expr(right);
            }
            Expr::UnaryExpr { op: _, expr } => {
                self.analyse_expr(expr);
            }
            Expr::ListLiteral(items) => {
                for item in items { self.analyse_expr(item); }
            }
            Expr::CastExpr { expr, to } => {
                self.analyse_expr(expr);  // add this!
                if let Some(from_ty) = self.infer_type(expr) {
                    let (valid, warning) = self.cast_valid(&from_ty, to);
                    if let Some(w) = warning {
                        self.errors.push(AnalysisError::Warning(w));
                    }
                    if !valid {
                        self.errors.push(AnalysisError::Error(format!(
                            "invalid cast from {} to {}: cannot cast '{}' to {}",
                            type_to_str(&from_ty), type_to_str(to),
                            expr_to_str(expr), type_to_str(to)
                        )));
                    }
                }
            }
            _ => {println!("Warning: undefined analysis behaviour for: {}", expr_to_str(expr))}
        }
    }

    fn infer_type(&mut self, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::Literal(lit) => Some(match lit {
                Literal::Int(_)    => Type::Int,
                Literal::Float(_)  => Type::Float,
                Literal::Double(_) => Type::Double,
                Literal::Str(_)    => Type::Str,
                Literal::Bool(_)   => Type::Bool,
                Literal::Null      => return None,
            }),
            Expr::Identifier(name) => {
                self.vars.get(name).cloned()
            }
            Expr::BinaryExpr { left, op, right } => {
                let lt = self.infer_type(left)?;
                let rt = self.infer_type(right)?;
                self.infer_binary_type(&lt, op, &rt)
            }
            Expr::ListLiteral(items) => {
                if items.is_empty() {
                    return Some(Type::List(None));
                }
                let mut element_types: Vec<Type> = Vec::new();
                for item in items.iter() {
                    if let Some(ty) = self.infer_type(&item.clone()) {
                        if !element_types.iter().any(|t| self.types_compatible(t, &ty) || self.types_compatible(&ty, t)) {
                            element_types.push(ty);
                        }
                    }
                }
                // coerce numeric types to widest
                let coerced = coerce_numeric_types(element_types);
                if coerced.is_empty() {
                    Some(Type::List(None))
                } else {
                    Some(Type::List(Some(coerced)))
                }
            }
            Expr::CastExpr { expr, to } => {
                if let Some(from_ty) = self.infer_type(expr) {
                    let (valid, warning) = self.cast_valid(&from_ty, to);
                    if let Some(w) = warning {
                        self.errors.push(AnalysisError::Warning(w));
                    }
                    if !valid {
                        self.errors.push(AnalysisError::Error(format!(
                            "invalid cast from {} to {}: {:?}",
                            type_to_str(&from_ty), type_to_str(to), expr_to_str(expr)
                        )));
                    }
                }
                Some(to.clone())
            }
            _ => None,
        }
    }

    fn infer_binary_type(&self, left: &Type, op: &BinaryOp, right: &Type) -> Option<Type> {
        match op {
            BinaryOp::Add | BinaryOp::Sub |
            BinaryOp::Mul | BinaryOp::Exp => {
                match (left, right) {
                    (Type::Double, _) | (_, Type::Double) => Some(Type::Double),
                    (Type::Float, _)  | (_, Type::Float)  => Some(Type::Float),
                    (Type::Int, Type::Int)                 => Some(Type::Int),
                    _ => None,
                }
            }
            BinaryOp::Div => Some(Type::Float),
            BinaryOp::Equal    | BinaryOp::NotEqual  |
            BinaryOp::Less     | BinaryOp::Greater   |
            BinaryOp::LessEqual| BinaryOp::GreaterEqual |
            BinaryOp::And      | BinaryOp::Or        => Some(Type::Bool),
            _ => None,
        }
    }

    fn types_compatible(&self, expected: &Type, actual: &Type) -> bool {
        match (expected, actual) {
            // exact match
            (Type::Int,    Type::Int)    => true,
            (Type::Float,  Type::Float)  => true,
            (Type::Double, Type::Double) => true,
            (Type::Str,    Type::Str)    => true,
            (Type::Bool,   Type::Bool)   => true,
            // int -> float/double implicit
            (Type::Float,  Type::Int)    => true,
            (Type::Double, Type::Int)    => true,
            (Type::Double, Type::Float)  => true,
            // untyped list compatible with any list
            (Type::List(_), Type::List(None)) => true,
            (Type::List(None), Type::List(_)) => true,
            // typed list
            (Type::List(Some(expected)), Type::List(Some(actual))) => {
                // each actual element type must match at least one expected type
                actual.iter().all(|actual_ty| {
                    expected.iter().any(|expected_ty| {
                        self.types_compatible(expected_ty, actual_ty)
                    })
                })
            }
            // user types
            (Type::UserType(a), Type::UserType(b)) => a == b,
            // generics are compatible with anything
            (Type::Generic(_), _) | (_, Type::Generic(_)) => true,
            _ => false,
        }
    }

    fn cast_valid(&self, from: &Type, to: &Type) -> (bool, Option<String>) {
        match (from, to) {
            (Type::Int,    Type::Float)  => (true, None),
            (Type::Int,    Type::Double) => (true, None),
            (Type::Int,    Type::Str)    => (true, None),
            (Type::Float,  Type::Int)    => (true, Some(format!(
                "cast from float to int will auto-round at runtime"
            ))),
            (Type::Float,  Type::Double) => (true, None),
            (Type::Float,  Type::Str)    => (true, None),
            (Type::Double, Type::Int)    => (true, Some(format!(
                "cast from double to int will auto-round at runtime"
            ))),
            (Type::Double, Type::Float)  => (true, Some(format!(
                "cast from double to float may lose precision at runtime"
            ))),
            (Type::Double, Type::Str)    => (true, None),
            (Type::Str,    Type::Int)    => (true, Some(format!(
                "cast from str to int may fail at runtime if str is not a valid int"
            ))),
            (Type::Str,    Type::Float)  => (true, Some(format!(
                "cast from str to float may fail at runtime if str is not a valid float"
            ))),
            (Type::Bool,   Type::Str)    => (true, None),
            (a, b) if a == b            => (true, Some(format!(
                "redundant cast from {} to {} — types are already the same",
                type_to_str(a), type_to_str(b)
            ))),
            (Type::Generic(_), _) | (_, Type::Generic(_)) => (true, None),
            (Type::UserType(a), Type::UserType(b)) if a == b => (true, None),
            _ => (false, None),
        }
    }

    fn inherits_from(&self, class_name: &str, target: &str) -> bool {
        if class_name == target { return true; }
        match self.classes.get(class_name) {
            Some(c) => match &c.parent {
                Some(parent) => self.inherits_from(parent, target),
                None => false,
            }
            None => false,
        }
    }
}

pub fn type_to_str(ty: &Type) -> String {
    match ty {
        Type::Int              => "int".to_string(),
        Type::Float            => "float".to_string(),
        Type::Double           => "double".to_string(),
        Type::Str              => "str".to_string(),
        Type::Bool             => "bool".to_string(),
        Type::List(None)       => "list".to_string(),
        Type::List(Some(types)) => {
            let inner = types.iter().map(type_to_str).collect::<Vec<_>>().join(" | ");
            format!("list<{}>", inner)
        }
        Type::UserType(n)      => n.clone(),
        Type::Generic(n)       => n.clone(),
    }
}

fn expr_to_str(expr: &Expr) -> String {
    match expr {
        Expr::Identifier(name)     => name.clone(),
        Expr::Literal(Literal::Int(n))    => n.to_string(),
        Expr::Literal(Literal::Float(f))  => format!("{}f", f),
        Expr::Literal(Literal::Double(d)) => format!("{}d", d),
        Expr::Literal(Literal::Str(s))    => format!("\"{}\"", s),
        Expr::Literal(Literal::Bool(b))   => b.to_string(),
        Expr::Literal(Literal::Null)      => "null".to_string(),
        Expr::CastExpr { expr, to }       => format!("{}<{}>", expr_to_str(expr), type_to_str(to)),
        Expr::FunctionCall { name, args: _ } => format!("{}(...)", name),
        Expr::MethodCall { object, method, .. } => format!("{}.{}(...)", expr_to_str(object), method),
        Expr::IndexExpr { object, index } => format!("{}[{}]", expr_to_str(object), expr_to_str(index)),
        _ => "<expr>".to_string(),
    }
}

pub fn coerce_numeric_types(types: Vec<Type>) -> Vec<Type> {
    let has_double = types.iter().any(|t| matches!(t, Type::Double));
    let has_float  = types.iter().any(|t| matches!(t, Type::Float));
    let has_int    = types.iter().any(|t| matches!(t, Type::Int));
    let non_numeric: Vec<Type> = types.into_iter()
        .filter(|t| !matches!(t, Type::Int | Type::Float | Type::Double))
        .collect();

    let mut result = non_numeric;

    // widen to the broadest numeric type present
    if has_double {
        result.push(Type::Double);
    } else if has_float {
        result.push(Type::Float);
    } else if has_int {
        result.push(Type::Int);
    }

    result
}