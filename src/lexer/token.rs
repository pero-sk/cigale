#[derive(Debug, PartialEq)]
pub enum Token {
    // literals
    IntLiteral(i64),
    FloatLiteral(f32),
    DoubleLiteral(f64),
    StringLiteral(String),
    BoolLiteral(bool),
    Null,

    // keywords -- types
    Int,
    Float,
    Double,
    Ref,
    Str,
    Bool,
    // keywords -- functions/classes
    Func,
    Class,
    Inst,
    Of,
    // keywords -- modifiers
    Public,
    Private,
    Protected,
    Blank,
    Impl,
    Static,
    // keywords -- control flow
    If,
    Else,
    For,
    Foreach,
    While,
    Break,
    Continue,
    Return,
    Match,
    // keywords -- other/misc
    Import,
    Typeof,
    As,


    // delimiters
    LParen, RParen, // ( )
    LBrace, RBrace, // { }
    LSBracket, RSBracket, // [ ]
    Comma, // ,
    Semicolon, // ;
    Dot, // .
    Arrow, // ->

    // operators
    Plus, // +
    Minus, // -
    Star, // *
    FSlash, // /
    Squiggle, // ~
    Percent, // %
    DStar, // **
    // operators -- comparison
    DEqual, // ==
    NotEqual, // !=
    Lesser, // <
    Greater, // >
    LesserEqual, // <=
    GreaterEqual, // >=
    // operators -- logical
    DAmp, // &&
    DPipe, // ||
    Not, // !
    // operators -- bitwise
    XOR, // ^
    AND, // &
    OR, // |
    LBS, // <<
    RBS, // >>
    // operators -- assignment
    Assign, // =
    PlusAssign, // +=
    MinusAssign, // -=
    StarAssign, // *=
    FSlashAssign, // /=
    SquiggleAssign, // ~=
    PercentAssign, // %=
    DStarAssign, // **=
    XORAssign, // ^=
    ANDAssign, // &=
    ORAssign, // |=
    LBSAssign, // <<=
    RBSAssign, // >>=
    // operators -- misc
    DollarSign, // $
    Underscore, // _
    DQuestion, // ??


    // Identifiers
    Identifier(String)
}