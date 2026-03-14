pub mod token;
use token::Token;


pub struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    fn current(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.current();
        self.pos += 1;
        c
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some(c) = self.current() {
            match c {
                ' ' | '\t' | '\n' | '\r' => { self.advance(); }
                '(' => { tokens.push(Token::LParen); self.advance(); }
                ')' => { tokens.push(Token::RParen); self.advance(); }
                '{' => { tokens.push(Token::LBrace); self.advance(); }
                '}' => { tokens.push(Token::RBrace); self.advance(); }
                '[' => { tokens.push(Token::LSBracket); self.advance(); }
                ']' => { tokens.push(Token::RSBracket); self.advance(); }
                ',' => { tokens.push(Token::Comma); self.advance(); }
                ';' => { tokens.push(Token::Semicolon); self.advance(); }
                '.' => { tokens.push(Token::Dot); self.advance(); }
                '$' => { tokens.push(Token::DollarSign); self.advance(); }
                '_' => { tokens.push(Token::Underscore); self.advance(); }
                '/' => { if let Some(t) = self.lex_slash() { tokens.push(t); } }
                '"' => { if let Some(t) = self.lex_string() { tokens.push(t); } }
                '0'..='9' => { if let Some(t) = self.lex_number() { tokens.push(t); } }
                'a'..='z' | 'A'..='Z' => { if let Some(t) = self.lex_identifier() { tokens.push(t); } }
                '-' => { if let Some(t) = self.lex_minus() { tokens.push(t); } }
                '*' => { if let Some(t) = self.lex_star() { tokens.push(t); } }
                '<' => { if let Some(t) = self.lex_lesser() { tokens.push(t); } }
                '>' => { if let Some(t) = self.lex_greater() { tokens.push(t); } }
                '=' => { if let Some(t) = self.lex_equal() { tokens.push(t); } }
                '!' => { if let Some(t) = self.lex_not() { tokens.push(t); } }
                '+' => { if let Some(t) = self.lex_plus() { tokens.push(t); } }
                '~' => { if let Some(t) = self.lex_squiggle() { tokens.push(t); } }
                '%' => { if let Some(t) = self.lex_percent() { tokens.push(t); } }
                '&' => { if let Some(t) = self.lex_amp() { tokens.push(t); } }
                '|' => { if let Some(t) = self.lex_pipe() { tokens.push(t); } }
                '^' => { if let Some(t) = self.lex_xor() { tokens.push(t); } }
                '?' => { if let Some(t) = self.lex_question() { tokens.push(t); } }
                _ => { self.advance(); }
            }
        }
        tokens
    }

    fn lex_slash(&mut self) -> Option<Token> {
        self.advance(); // consume '/'
        match self.current() {
            Some('=') => { self.advance(); Some(Token::FSlashAssign) }
            Some('/') => {
                // single line comment -- consume until newline
                while let Some(c) = self.current() {
                    if c == '\n' { break; }
                    self.advance();
                }
                None // no token for comments
            }
            Some('"') => {
                // multiline comment /" ... "/
                self.advance(); // consume '"'
                loop {
                    match self.current() {
                        Some('"') if self.peek() == Some('/') => {
                            self.advance(); // consume '"'
                            self.advance(); // consume '/'
                            break;
                        }
                        None => break, // unterminated comment
                        _ => { self.advance(); }
                    }
                }
                None // no token for comments
            }
            _ => Some(Token::FSlash)
        }
    }

    fn lex_number(&mut self) -> Option<Token> {
        let mut num = String::new();
        let mut is_decimal = false;

        while let Some(c) = self.current() {
            if c.is_ascii_digit() {
                num.push(c);
                self.advance();
            } else if c == '.' && !is_decimal {
                is_decimal = true;
                num.push(c);
                self.advance();
            } else {
                break;
            }
        }

        match self.current() {
            Some('f') => { self.advance(); Some(Token::FloatLiteral(num.parse().unwrap())) }
            Some('d') => { self.advance(); Some(Token::DoubleLiteral(num.parse().unwrap())) }
            _ => {
                if is_decimal {
                    Some(Token::DoubleLiteral(num.parse().unwrap()))
                } else {
                    Some(Token::IntLiteral(num.parse().unwrap()))
                }
            }
        }
    }

    fn lex_identifier(&mut self) -> Option<Token> {
        let mut ident = String::new();
        while let Some(c) = self.current() {
            if c.is_alphanumeric() || c == '_' {
                ident.push(c);
                self.advance();
            } else {
                break;
            }
        }
        Some(match ident.as_str() {
            "int"       => Token::Int,
            "float"     => Token::Float,
            "double"    => Token::Double,
            "str"       => Token::Str,
            "bool"      => Token::Bool,
            "func"      => Token::Func,
            "class"     => Token::Class,
            "inst"      => Token::Inst,
            "of"        => Token::Of,
            "public"    => Token::Public,
            "private"   => Token::Private,
            "protected" => Token::Protected,
            "blank"     => Token::Blank,
            "impl"      => Token::Impl,
            "static"    => Token::Static,
            "if"        => Token::If,
            "else"      => Token::Else,
            "for"       => Token::For,
            "foreach"   => Token::Foreach,
            "while"     => Token::While,
            "break"     => Token::Break,
            "continue"  => Token::Continue,
            "return"    => Token::Return,
            "match"     => Token::Match,
            "import"    => Token::Import,
            "typeof"    => Token::Typeof,
            "as"        => Token::As,
            "true"      => Token::BoolLiteral(true),
            "false"     => Token::BoolLiteral(false),
            "null"      => Token::Null,
            _           => Token::Identifier(ident)
        })
    }

    fn lex_string(&mut self) -> Option<Token> {
        self.advance(); // consume opening '"'
        let mut s = String::new();
        loop {
            match self.current() {
                Some('"') => { self.advance(); break; }
                Some('\\') => {
                    self.advance();
                    match self.current() {
                        Some('n')  => { s.push('\n'); self.advance(); }
                        Some('t')  => { s.push('\t'); self.advance(); }
                        Some('"')  => { s.push('"');  self.advance(); }
                        Some('\\') => { s.push('\\'); self.advance(); }
                        _ => {}
                    }
                }
                Some(c) => { s.push(c); self.advance(); }
                None => break, // unterminated string
            }
        }
        Some(Token::StringLiteral(s))
    }

    fn lex_minus(&mut self) -> Option<Token> {
        self.advance(); // consume '-'
        Some(match self.current() {
            Some('>') => { self.advance(); Token::Arrow }
            Some('=') => { self.advance(); Token::MinusAssign }
            _ => Token::Minus
        })
    }

        fn lex_star(&mut self) -> Option<Token> {
        self.advance(); // consume '*'
        Some(match self.current() {
            Some('*') => {
                self.advance(); // consume second '*'
                match self.current() {
                    Some('=') => { self.advance(); Token::DStarAssign }
                    _ => Token::DStar
                }
            }
            Some('=') => { self.advance(); Token::StarAssign }
            _ => Token::Star
        })
    }

    fn lex_lesser(&mut self) -> Option<Token> {
        self.advance(); // consume '<'
        Some(match self.current() {
            Some('<') => {
                self.advance(); // consume second '<'
                match self.current() {
                    Some('=') => { self.advance(); Token::LBSAssign }
                    _ => Token::LBS
                }
            }
            Some('=') => { self.advance(); Token::LesserEqual }
            _ => Token::Lesser
        })
    }

    fn lex_greater(&mut self) -> Option<Token> {
        self.advance(); // consume '>'
        Some(match self.current() {
            Some('>') => {
                self.advance(); // consume second '>'
                match self.current() {
                    Some('=') => { self.advance(); Token::RBSAssign }
                    _ => Token::RBS
                }
            }
            Some('=') => { self.advance(); Token::GreaterEqual }
            _ => Token::Greater
        })
    }

    fn lex_equal(&mut self) -> Option<Token> {
        self.advance(); // consume '='
        Some(match self.current() {
            Some('=') => { self.advance(); Token::DEqual }
            _ => Token::Assign
        })
    }

    fn lex_not(&mut self) -> Option<Token> {
        self.advance(); // consume '!'
        Some(match self.current() {
            Some('=') => { self.advance(); Token::NotEqual }
            _ => Token::Not
        })
    }
    fn lex_plus(&mut self) -> Option<Token> {
        self.advance(); // consume '+'
        Some(match self.current() {
            Some('=') => { self.advance(); Token::PlusAssign }
            _ => Token::Plus
        })
    }

    fn lex_squiggle(&mut self) -> Option<Token> {
        self.advance(); // consume '~'
        Some(match self.current() {
            Some('=') => { self.advance(); Token::SquiggleAssign }
            _ => Token::Squiggle
        })
    }

    fn lex_percent(&mut self) -> Option<Token> {
        self.advance(); // consume '%'
        Some(match self.current() {
            Some('=') => { self.advance(); Token::PercentAssign }
            _ => Token::Percent
        })
    }

    fn lex_amp(&mut self) -> Option<Token> {
        self.advance(); // consume '&'
        Some(match self.current() {
            Some('&') => { self.advance(); Token::DAmp }
            Some('=') => { self.advance(); Token::ANDAssign }
            _ => Token::AND
        })
    }

    fn lex_pipe(&mut self) -> Option<Token> {
        self.advance(); // consume '|'
        Some(match self.current() {
            Some('|') => { self.advance(); Token::DPipe }
            Some('=') => { self.advance(); Token::ORAssign }
            _ => Token::OR
        })
    }

    fn lex_xor(&mut self) -> Option<Token> {
        self.advance(); // consume '^'
        Some(match self.current() {
            Some('=') => { self.advance(); Token::XORAssign }
            _ => Token::XOR
        })
    }

    fn lex_question(&mut self) -> Option<Token> {
        self.advance(); // consume '?'
        Some(match self.current() {
            Some('?') => { self.advance(); Token::DQuestion }
            _ => {
                // single '?' is not a valid token in Cigale, skip it
                None?
            }
        })
    }
}