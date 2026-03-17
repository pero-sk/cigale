#![allow(unused)]
#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64),
    Float(f32),
    Double(f64),
    Str(String),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Double,
    Str,
    Bool,
    List(Option<Vec<Type>>),    // list or list<str|int>
    UserType(String),           // class inst types
    Generic(String),            // T, U, V etc.
    Ref(Box<Type>),             // ref<T> type
}

#[derive(Debug, Clone)]
pub struct FuncModifiers {
    pub access: AccessModifier,
    pub is_blank: bool,
    pub is_impl: bool,
    pub is_static: bool,
}

#[derive(Debug, Clone)]
pub enum AccessModifier {
    Public,
    Private,
    Protected,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Literal),
    Identifier(String),
    BinaryExpr {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    UnaryExpr {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    FunctionCall {
        name: String,
        args: Vec<Expr>,
    },
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    IndexExpr {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    CastExpr {
        expr: Box<Expr>,
        to: Type,
    },
    NullishExpr {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    ListLiteral(Vec<Expr>),
    InterpolatedString(Vec<StringPart>),
    TypeofExpr {
        ty: Type,
        expr: Box<Expr>,
    },
    
    RefExpr(Box<Expr>),
    DerefExpr(Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum StringPart {
    Literal(String),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    // arithmetic
    Add, Sub, Mul, Div,
    Modulo, Percent, Exp,
    // comparison
    Equal, NotEqual,
    Less, Greater,
    LessEqual, GreaterEqual,
    // logical
    And, Or,
    // bitwise
    XOR, AND, OR,
    BitShiftLeft, BitShiftRight,
    // assignment
    Assign,
    AddAssign, SubAssign, MulAssign, DivAssign,
    ModuloAssign, PercentAssign, ExpAssign,
    XORAssign, ANDAssign, ORAssign,
    BitShiftLeftAssign, BitShiftRightAssign,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub patterns: Vec<Expr>,    // colour.RED | colour.GREEN
    pub body: Vec<Stmt>,
    pub is_default: bool,       // _ -> { ... }
}

#[derive(Debug, Clone)]
pub struct FunctionParam {
    pub name: String,
    pub ty: Option<Type>,       // None if untyped
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub modifiers: FuncModifiers,
    pub param_types: Vec<Type>,
    pub params: Vec<FunctionParam>,
    pub return_type: Option<Type>,
    pub body: Option<Vec<Stmt>>,    // None if blank
}

#[derive(Debug, Clone)]
pub struct ClassDecl {
    pub name: String,
    pub is_inst: bool,
    pub parent: Option<String>,
    pub body: Vec<FunctionDecl>,
    pub fields: Vec<Stmt>
}



#[derive(Debug, Clone)]
pub enum Stmt {
    // imports
    ImportStatement {
        path: Vec<String>,          // ["stdl", "maths"]
        alias: Option<String>,      // as m
        items: Option<Vec<(String, Option<String>)>>, // { pi, add as a } -- None = whole module, Some([]) = *
    },
    // declarations
    FunctionDeclaration(FunctionDecl),
    ClassDeclaration(ClassDecl),

    VariableDeclaration {
        name: String,
        ty: Option<Type>,           // None if untyped
        is_static: bool,
        access: Option<AccessModifier>,
        value: Option<Expr>,
    },
    // control flow
    ReturnStatement(Option<Expr>),
    IfStatement {
        condition: Expr,
        body: Vec<Stmt>,
        else_ifs: Vec<(Expr, Vec<Stmt>)>,
        else_body: Option<Vec<Stmt>>,
    },
    ForStatement {
        init: Box<Stmt>,
        condition: Expr,
        step: Box<Stmt>,
        body: Vec<Stmt>,
    },
    ForeachStatement {
        ty: Option<Type>,
        name: String,
        iterable: Expr,
        body: Vec<Stmt>,
    },
    WhileStatement {
        condition: Expr,
        body: Vec<Stmt>,
    },
    MatchStatement {
        value: Expr,
        arms: Vec<MatchArm>,
    },
    BreakStatement,
    ContinueStatement,
    StaticBlock(Vec<Stmt>),
    ExpressionStatement(Expr),
    EnumDeclaration {
        name: String,
        variants: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub struct Program {
    pub body: Vec<Stmt>,
}