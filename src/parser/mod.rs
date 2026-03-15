pub mod ast;
use crate::parser::ast::*;

use crate::lexer::token::Token;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    // unused
    // fn peek(&self) -> Option<&Token> {
    //     self.tokens.get(self.pos + 1)
    // }

    fn advance(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.pos);
        self.pos += 1;
        t
    }

    fn expect(&mut self, expected: &Token, from: &'static str) -> Result<(), String> {
        match self.current() {
            Some(t) if t == expected => { self.advance(); Ok(()) }
            Some(t) => Err(format!("[{}] expected {:?} but got {:?} [at pos: {:?}] [previous: {:?}]", from, expected, t, self.pos, self.tokens[self.pos-1])),
            None => Err(format!("[{}] expected {:?} but got EOF", from, expected)),
        }
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut body = Vec::new();
        while self.current().is_some() {
            body.push(self.parse_stmt()?);
        }
        Ok(Program { body })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.current() {
            Some(Token::Import)   => self.parse_import(),
            Some(Token::Func)     => self.parse_function(),
            Some(Token::Class)    => self.parse_class(),
            Some(Token::Inst)     => self.parse_enum(),
            Some(Token::Static)   => self.parse_static(),
            Some(Token::If)       => self.parse_if(),
            Some(Token::For)      => self.parse_for(),
            Some(Token::Foreach)  => self.parse_foreach(),
            Some(Token::While)    => self.parse_while(),
            Some(Token::Match)    => self.parse_match(),
            Some(Token::Return)   => self.parse_return(),
            Some(Token::Break) => {
                self.advance();
                self.expect(&Token::Semicolon, "parse_stmt::break")?;
                Ok(Stmt::BreakStatement)
            }
            Some(Token::Continue) => {
                self.advance();
                self.expect(&Token::Semicolon, "parse_stmt::continue")?;
                Ok(Stmt::ContinueStatement)
            }
            // primitive type keywords -> variable declaration
            Some(Token::Int)    | Some(Token::Float) |
            Some(Token::Double) | Some(Token::Str)   |
            Some(Token::Bool)   => self.parse_variable_decl(),
            // identifier -> could be variable declaration (user type) or expression
            Some(Token::Identifier(_)) => {
                // peek ahead to detect: TypeName varName = ...
                // if token after identifier is also an identifier, it's a variable declaration
                if self.is_variable_decl() {
                    self.parse_variable_decl()
                } else {
                    let expr = self.parse_expr()?;
                    self.expect(&Token::Semicolon, "parse_stmt::expr")?;
                    Ok(Stmt::ExpressionStatement(expr))
                }
            }
            _ => {
                let expr = self.parse_expr()?;
                self.expect(&Token::Semicolon, "parse_stmt")?;
                Ok(Stmt::ExpressionStatement(expr))
            }
        }
    }

    fn parse_import(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'import'

        let mut path = Vec::new();
        match self.current() {
            Some(Token::Identifier(name)) => {
                path.push(name.clone());
                self.advance();
            }
            Some(t) => return Err(format!("expected module name but got {:?}", t)),
            None => return Err("expected module name but got EOF".to_string()),
        }
        while let Some(Token::Dot) = self.current() {
            self.advance();
            match self.current() {
                Some(Token::Identifier(name)) => {
                    path.push(name.clone());
                    self.advance();
                }
                Some(t) => return Err(format!("expected module name but got {:?}", t)),
                None => return Err("expected module name but got EOF".to_string()),
            }
        }

        match self.current() {
            Some(Token::As) => {
                self.advance();
                match self.current() {
                    Some(Token::Identifier(alias)) => {
                        let alias = alias.clone();
                        self.advance();
                        self.expect(&Token::Semicolon, "parse_import::as")?;
                        Ok(Stmt::ImportStatement {
                            path,
                            alias: Some(alias),
                            items: None,
                        })
                    }
                    Some(t) => Err(format!("expected alias but got {:?}", t)),
                    None => Err("expected alias but got EOF".to_string()),
                }
            }
            Some(Token::LBrace) => {
                self.advance();
                let mut items = Vec::new();
                if let Some(Token::Star) = self.current() {
                    self.advance();
                    self.expect(&Token::RBrace, "parse_import::wildcard::rbrace")?;
                    self.expect(&Token::Semicolon, "parse_import::wildcard::semicolon")?;
                    return Ok(Stmt::ImportStatement {
                        path,
                        alias: None,
                        items: Some(Vec::new()),
                    });
                }
                loop {
                    match self.current() {
                        Some(Token::Identifier(name)) => {
                            let name = name.clone();
                            self.advance();
                            let alias = if let Some(Token::As) = self.current() {
                                self.advance();
                                match self.current() {
                                    Some(Token::Identifier(a)) => {
                                        let a = a.clone();
                                        self.advance();
                                        Some(a)
                                    }
                                    Some(t) => return Err(format!("expected alias but got {:?}", t)),
                                    None => return Err("expected alias but got EOF".to_string()),
                                }
                            } else {
                                None
                            };
                            items.push((name, alias));
                            match self.current() {
                                Some(Token::Comma) => { self.advance(); }
                                Some(Token::RBrace) => break,
                                Some(t) => return Err(format!("expected ',' or '}}' but got {:?}", t)),
                                None => return Err("expected ',' or '}}' but got EOF".to_string()),
                            }
                        }
                        Some(t) => return Err(format!("expected item name but got {:?}", t)),
                        None => return Err("expected item name but got EOF".to_string()),
                    }
                }
                self.expect(&Token::RBrace, "parse_import::selective::rbrace")?;
                self.expect(&Token::Semicolon, "parse_import::selective::semicolon")?;
                Ok(Stmt::ImportStatement {
                    path,
                    alias: None,
                    items: Some(items),
                })
            }
            Some(t) => Err(format!("expected 'as' or '{{' but got {:?}", t)),
            None => Err("expected 'as' or '{{' but got EOF".to_string()),
        }
    }

    fn parse_function(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'func'

        let mut access = AccessModifier::Public;
        let mut is_blank = false;
        let mut is_impl = false;
        let mut is_static = false;

        loop {
            match self.current() {
                Some(Token::Public)    => { access = AccessModifier::Public;    self.advance(); }
                Some(Token::Private)   => { access = AccessModifier::Private;   self.advance(); }
                Some(Token::Protected) => { access = AccessModifier::Protected; self.advance(); }
                Some(Token::Blank)     => { is_blank = true;  self.advance(); }
                Some(Token::Impl)      => { is_impl = true;   self.advance(); }
                Some(Token::Static)    => { is_static = true; self.advance(); }
                _ => break,
            }
        }

        if is_blank && matches!(access, AccessModifier::Private) {
            return Err("func blank cannot be private".to_string());
        }

        let name = match self.current() {
            Some(Token::Identifier(n)) => { let n = n.clone(); self.advance(); n }
            Some(t) => return Err(format!("expected function name but got {:?}", t)),
            None => return Err("expected function name but got EOF".to_string()),
        };

        let mut param_types = Vec::new();
        if let Some(Token::Lesser) = self.current() {
            self.advance();
            while let Some(t) = self.current() {
                if matches!(t, Token::Greater) { break; }
                param_types.push(self.parse_type()?);
                match self.current() {
                    Some(Token::Comma) => { self.advance(); }
                    Some(Token::Greater) => break,
                    Some(t) => return Err(format!("expected ',' or '>' but got {:?}", t)),
                    None => return Err("expected ',' or '>' but got EOF".to_string()),
                }
            }
            self.expect(&Token::Greater, "parse_function::type_params")?;
        }

        self.expect(&Token::LParen, "parse_function::params::lparen")?;
        let mut params = Vec::new();
        while let Some(t) = self.current() {
            if matches!(t, Token::RParen) { break; }
            match self.current() {
                Some(Token::Identifier(n)) => {
                    let n = n.clone();
                    self.advance();
                    params.push(FunctionParam { name: n, ty: None });
                }
                Some(t) => return Err(format!("expected param name but got {:?}", t)),
                None => return Err("expected param name but got EOF".to_string()),
            }
            match self.current() {
                Some(Token::Comma) => { self.advance(); }
                Some(Token::RParen) => break,
                Some(t) => return Err(format!("expected ',' or ')' but got {:?}", t)),
                None => return Err("expected ',' or ')' but got EOF".to_string()),
            }
        }
        self.expect(&Token::RParen, "parse_function::params::rparen")?;

        let return_type = if let Some(Token::Arrow) = self.current() {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        if is_blank {
            self.expect(&Token::Semicolon, "parse_function::blank::semicolon")?;
            return Ok(Stmt::FunctionDeclaration(FunctionDecl {
                name,
                modifiers: FuncModifiers { access, is_blank, is_impl, is_static },
                param_types,
                params,
                return_type,
                body: None,
            }));
        }

        let body = self.parse_block()?;

        Ok(Stmt::FunctionDeclaration(FunctionDecl {
            name,
            modifiers: FuncModifiers { access, is_blank, is_impl, is_static },
            param_types,
            params,
            return_type,
            body: Some(body),
        }))
    }

    fn parse_class(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'class'

        let is_inst = if let Some(Token::Inst) = self.current() {
            self.advance();
            true
        } else {
            false
        };

        let parent = if let Some(Token::Of) = self.current() {
            self.advance();
            self.expect(&Token::Lesser, "parse_class::of::langle")?;
            let parent = match self.current() {
                Some(Token::Identifier(n)) => { let n = n.clone(); self.advance(); n }
                Some(t) => return Err(format!("expected parent type but got {:?}", t)),
                None => return Err("expected parent type but got EOF".to_string()),
            };
            self.expect(&Token::Greater, "parse_class::of::rangle")?;
            Some(parent)
        } else {
            None
        };

        let name = match self.current() {
            Some(Token::Identifier(n)) => { let n = n.clone(); self.advance(); n }
            Some(t) => return Err(format!("expected class name but got {:?}", t)),
            None => return Err("expected class name but got EOF".to_string()),
        };

        self.expect(&Token::LBrace, "parse_class::body::lbrace")?;
        let mut body: Vec<FunctionDecl> = Vec::new();
        let mut fields: Vec<Stmt> = Vec::new();
        while let Some(t) = self.current() {
            if matches!(t, Token::RBrace) {
                break;
            }

            match self.current() {
                Some(Token::Func) => {
                    match self.parse_function()? {
                        Stmt::FunctionDeclaration(f) => {
                            if f.modifiers.is_blank && !is_inst {
                                return Err("func blank only allowed in class inst".to_string());
                            }
                            body.push(f);
                        }
                        _ => unreachable!(),
                    }
                }
                Some(Token::Int)    | Some(Token::Float) |
                Some(Token::Double) | Some(Token::Str)   |
                Some(Token::Bool)   => {
                    fields.push(self.parse_variable_decl()?);
                }
                Some(Token::Identifier(_)) if self.is_variable_decl() => {
                    fields.push(self.parse_variable_decl()?);
                }

                Some(t) => {
                    return Err(format!("unexpected token in class body: {:?}", t));
                }

                None => {
                    return Err("unexpected EOF in class body".to_string());
                }
            }
        }
        self.expect(&Token::RBrace, "parse_class::body::rbrace")?;

        Ok(Stmt::ClassDeclaration(ClassDecl { name, is_inst, parent, body, fields }))
    }

    fn parse_enum(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'inst'

        let name = match self.current() {
            Some(Token::Identifier(n)) => { let n = n.clone(); self.advance(); n }
            Some(t) => return Err(format!("expected enum name but got {:?}", t)),
            None => return Err("expected enum name but got EOF".to_string()),
        };

        self.expect(&Token::LBrace, "parse_enum::lbrace")?;
        let mut variants = Vec::new();
        loop {
            match self.current() {
                Some(Token::Identifier(v)) => {
                    variants.push(v.clone());
                    self.advance();
                    match self.current() {
                        Some(Token::Comma) => { self.advance(); }
                        Some(Token::RBrace) => break,
                        Some(t) => return Err(format!("expected ',' or '}}' but got {:?}", t)),
                        None => return Err("expected ',' or '}}' but got EOF".to_string()),
                    }
                }
                Some(Token::RBrace) => break,
                Some(t) => return Err(format!("expected variant name but got {:?}", t)),
                None => return Err("expected variant name but got EOF".to_string()),
            }
        }
        self.expect(&Token::RBrace, "parse_enum::rbrace")?;

        Ok(Stmt::EnumDeclaration { name, variants })
    }

    fn parse_static(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'static'

        match self.current() {
            Some(Token::Int)   | Some(Token::Float) |
            Some(Token::Double)| Some(Token::Str)   |
            Some(Token::Bool)  => {
                let mut decl = self.parse_variable_decl()?;
                if let Stmt::VariableDeclaration { ref mut is_static, .. } = decl {
                    *is_static = true;
                }
                Ok(decl)
            }
            Some(Token::LBrace) => {
                let body = self.parse_block()?;
                Ok(Stmt::StaticBlock(body))
            }
            Some(t) => Err(format!("expected type or '{{' after static but got {:?}", t)),
            None => Err("expected type or '{{' after static but got EOF".to_string()),
        }
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(&Token::LBrace, "parse_block::lbrace")?;
        let mut stmts = Vec::new();
        while let Some(t) = self.current() {
            if matches!(t, Token::RBrace) { break; }
            stmts.push(self.parse_stmt()?);
        }
        self.expect(&Token::RBrace, "parse_block::rbrace")?;
        Ok(stmts)
    }

    fn parse_type(&mut self) -> Result<Type, String> {
        match self.current() {
            Some(Token::Int)    => { self.advance(); Ok(Type::Int) }
            Some(Token::Float)  => { self.advance(); Ok(Type::Float) }
            Some(Token::Double) => { self.advance(); Ok(Type::Double) }
            Some(Token::Str)    => { self.advance(); Ok(Type::Str) }
            Some(Token::Bool)   => { self.advance(); Ok(Type::Bool) }
            Some(Token::Identifier(n)) => {
                let n = n.clone();
                self.advance();
                // single uppercase letter = generic
                if n.len() == 1 && n.chars().next().unwrap().is_uppercase() {
                    return Ok(Type::Generic(n));
                }
                // list<T> or list<T|U> or untyped list
                if n == "list" {
                    if let Some(Token::Lesser) = self.current() {
                        self.advance(); // consume '<'
                        let mut types = Vec::new();
                        loop {
                            types.push(self.parse_type()?);
                            match self.current() {
                                Some(Token::OR) => { self.advance(); }
                                Some(Token::Greater) => break,
                                Some(t) => return Err(format!("expected '|' or '>' in list type but got {:?}", t)),
                                None => return Err("expected '|' or '>' in list type but got EOF".to_string()),
                            }
                        }
                        self.expect(&Token::Greater, "parse_type::list::rangle")?;
                        return Ok(Type::List(Some(types)));
                    }
                    return Ok(Type::List(None)); // untyped list
                }
                // generic user type e.g. result<int>
                if let Some(Token::Lesser) = self.current() {
                    self.advance();
                    let mut params = Vec::new();
                    loop {
                        params.push(self.parse_type()?);
                        match self.current() {
                            Some(Token::Comma) => { self.advance(); }
                            Some(Token::Greater) => break,
                            Some(t) => return Err(format!("expected ',' or '>' in type params but got {:?}", t)),
                            None => return Err("expected ',' or '>' in type params but got EOF".to_string()),
                        }
                    }
                    self.expect(&Token::Greater, "parse_type::user_type::rangle")?;
                    return Ok(Type::UserType(n));
                }
                Ok(Type::UserType(n))
            }
            Some(Token::LSBracket) => {
                self.advance();
                if let Some(Token::RSBracket) = self.current() {
                    self.advance();
                    Ok(Type::List(None))
                } else {
                    let mut types = Vec::new();
                    loop {
                        types.push(self.parse_type()?);
                        match self.current() {
                            Some(Token::OR) => { self.advance(); }
                            Some(Token::RSBracket) => break,
                            Some(t) => return Err(format!("expected '|' or ']' but got {:?}", t)),
                            None => return Err("expected '|' or ']' but got EOF".to_string()),
                        }
                    }
                    self.expect(&Token::RSBracket, "parse_type::list::rbracket")?;
                    Ok(Type::List(Some(types)))
                }
            }
            Some(t) => Err(format!("expected type but got {:?}", t)),
            None => Err("expected type but got EOF".to_string()),
        }
    }

    fn parse_if(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'if'

        self.expect(&Token::LParen, "parse_if::lparen")?;
        let condition = self.parse_expr()?;
        self.expect(&Token::RParen, "parse_if::rparen")?;
        let body = self.parse_block()?;

        let mut else_ifs = Vec::new();
        let mut else_body = None;

        loop {
            match self.current() {
                Some(Token::Else) => {
                    self.advance();
                    match self.current() {
                        Some(Token::If) => {
                            self.advance();
                            self.expect(&Token::LParen, "parse_if::else_if::lparen")?;
                            let cond = self.parse_expr()?;
                            self.expect(&Token::RParen, "parse_if::else_if::rparen")?;
                            let body = self.parse_block()?;
                            else_ifs.push((cond, body));
                        }
                        Some(Token::LBrace) => {
                            else_body = Some(self.parse_block()?);
                            break;
                        }
                        Some(t) => return Err(format!("expected 'if' or '{{' after else but got {:?}", t)),
                        None => return Err("expected 'if' or '{{' after else but got EOF".to_string()),
                    }
                }
                _ => break,
            }
        }

        Ok(Stmt::IfStatement { condition, body, else_ifs, else_body })
    }

    fn parse_for(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'for'

        self.expect(&Token::LParen, "parse_for::lparen")?;
        let init = self.parse_variable_decl_no_semi()?;
        self.expect(&Token::Comma, "parse_for::comma_1")?;
        let condition = self.parse_expr()?;
        self.expect(&Token::Comma, "parse_for::comma_2")?;
        let step = self.parse_expr()?;
        self.expect(&Token::RParen, "parse_for::rparen")?;

        let body = self.parse_block()?;

        Ok(Stmt::ForStatement {
            init: Box::new(init),
            condition,
            step: Box::new(Stmt::ExpressionStatement(step)),
            body,
        })
    }

    fn parse_foreach(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'foreach'

        self.expect(&Token::LParen, "parse_foreach::lparen")?;

        let (ty, name) = match self.current() {
            Some(Token::Int)    | Some(Token::Float) |
            Some(Token::Double) | Some(Token::Str)   |
            Some(Token::Bool)   => {
                let ty = self.parse_type()?;
                let name = match self.current() {
                    Some(Token::Identifier(n)) => { let n = n.clone(); self.advance(); n }
                    Some(t) => return Err(format!("expected variable name but got {:?}", t)),
                    None => return Err("expected variable name but got EOF".to_string()),
                };
                (Some(ty), name)
            }
            Some(Token::Identifier(n)) => {
                let name = n.clone();
                self.advance();
                (None, name)
            }
            Some(t) => return Err(format!("expected type or variable name but got {:?}", t)),
            None => return Err("expected type or variable name but got EOF".to_string()),
        };

        self.expect(&Token::Comma, "parse_foreach::comma")?;
        let iterable = self.parse_expr()?;
        self.expect(&Token::RParen, "parse_foreach::rparen")?;

        let body = self.parse_block()?;

        Ok(Stmt::ForeachStatement { ty, name, iterable, body })
    }

    fn parse_while(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'while'

        self.expect(&Token::LParen, "parse_while::lparen")?;
        let condition = self.parse_expr()?;
        self.expect(&Token::RParen, "parse_while::rparen")?;

        let body = self.parse_block()?;

        Ok(Stmt::WhileStatement { condition, body })
    }

    fn parse_match(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'match'

        self.expect(&Token::LParen, "parse_match::lparen")?;
        let value = self.parse_expr()?;
        self.expect(&Token::RParen, "parse_match::rparen")?;

        self.expect(&Token::LBrace, "parse_match::lbrace")?;
        let mut arms = Vec::new();

        while let Some(t) = self.current() {
            if matches!(t, Token::RBrace) { break; }

            if let Some(Token::Underscore) = self.current() {
                self.advance();
                self.expect(&Token::Arrow, "parse_match::default::arrow")?;
                let body = self.parse_block()?;
                arms.push(MatchArm {
                    patterns: Vec::new(),
                    body,
                    is_default: true,
                });
                break;
            }

            let mut patterns = Vec::new();
            loop {
                patterns.push(self.parse_comparison()?);
                match self.current() {
                    Some(Token::OR) => { self.advance(); }
                    Some(Token::Arrow) => break,
                    Some(t) => return Err(format!("expected '|' or '->' but got {:?}", t)),
                    None => return Err("expected '|' or '->' but got EOF".to_string()),
                }
            }

            self.expect(&Token::Arrow, "parse_match::arm::arrow")?;
            let body = self.parse_block()?;

            arms.push(MatchArm {
                patterns,
                body,
                is_default: false,
            });
        }

        self.expect(&Token::RBrace, "parse_match::rbrace")?;

        Ok(Stmt::MatchStatement { value, arms })
    }

    fn parse_return(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'return'

        let value = if let Some(Token::Semicolon) = self.current() {
            None
        } else {
            Some(self.parse_expr()?)
        };

        self.expect(&Token::Semicolon, "parse_return::semicolon")?;
        Ok(Stmt::ReturnStatement(value))
    }

    fn parse_variable_decl_no_semi(&mut self) -> Result<Stmt, String> {
        let ty = self.parse_type()?;

        let access = match self.current() {
            Some(Token::Public)    => { self.advance(); Some(AccessModifier::Public) }
            Some(Token::Private)   => { self.advance(); Some(AccessModifier::Private) }
            Some(Token::Protected) => { self.advance(); Some(AccessModifier::Protected) }
            _ => None,
        };

        let name = match self.current() {
            Some(Token::Identifier(n)) => { let n = n.clone(); self.advance(); n }
            Some(t) => return Err(format!("expected variable name but got {:?}", t)),
            None => return Err("expected variable name but got EOF".to_string()),
        };

        let value = if let Some(Token::Assign) = self.current() {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        // no semicolon consumed!

        Ok(Stmt::VariableDeclaration {
            name,
            ty: Some(ty),
            is_static: false,
            access,
            value,
        })
    }

    fn parse_variable_decl(&mut self) -> Result<Stmt, String> {
        let ty = self.parse_type()?;

        let access = match self.current() {
            Some(Token::Public)    => { self.advance(); Some(AccessModifier::Public) }
            Some(Token::Private)   => { self.advance(); Some(AccessModifier::Private) }
            Some(Token::Protected) => { self.advance(); Some(AccessModifier::Protected) }
            _ => None,
        };

        let name = match self.current() {
            Some(Token::Identifier(n)) => { let n = n.clone(); self.advance(); n }
            Some(t) => return Err(format!("expected variable name but got {:?}", t)),
            None => return Err("expected variable name but got EOF".to_string()),
        };

        let value = if let Some(Token::Assign) = self.current() {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        self.expect(&Token::Semicolon, "parse_variable_decl::semicolon")?;

        Ok(Stmt::VariableDeclaration {
            name,
            ty: Some(ty),
            is_static: false,
            access,
            value,
        })
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, String> {
        let left = self.parse_nullish()?;

        let op = match self.current() {
            Some(Token::Assign)          => Some(BinaryOp::Assign),
            Some(Token::PlusAssign)      => Some(BinaryOp::AddAssign),
            Some(Token::MinusAssign)     => Some(BinaryOp::SubAssign),
            Some(Token::StarAssign)      => Some(BinaryOp::MulAssign),
            Some(Token::FSlashAssign)    => Some(BinaryOp::DivAssign),
            Some(Token::SquiggleAssign)  => Some(BinaryOp::ModuloAssign),
            Some(Token::PercentAssign)   => Some(BinaryOp::PercentAssign),
            Some(Token::DStarAssign)     => Some(BinaryOp::ExpAssign),
            Some(Token::XORAssign)       => Some(BinaryOp::XORAssign),
            Some(Token::ANDAssign)       => Some(BinaryOp::ANDAssign),
            Some(Token::ORAssign)        => Some(BinaryOp::ORAssign),
            Some(Token::LBSAssign)       => Some(BinaryOp::BitShiftLeftAssign),
            Some(Token::RBSAssign)       => Some(BinaryOp::BitShiftRightAssign),
            _ => None,
        };

        if let Some(op) = op {
            self.advance();
            let right = self.parse_assignment()?;
            return Ok(Expr::BinaryExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
            });
        }

        Ok(left)
    }

    fn parse_nullish(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_or()?;
        while let Some(Token::DQuestion) = self.current() {
            self.advance();
            let right = self.parse_or()?;
            left = Expr::NullishExpr {
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;
        while let Some(Token::DPipe) = self.current() {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinaryExpr {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_or()?;
        while let Some(Token::DAmp) = self.current() {
            self.advance();
            let right = self.parse_bitwise_or()?;
            left = Expr::BinaryExpr {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_xor()?;
        while let Some(Token::OR) = self.current() {
            self.advance();
            let right = self.parse_xor()?;
            left = Expr::BinaryExpr {
                left: Box::new(left),
                op: BinaryOp::OR,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_xor(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_and()?;
        while let Some(Token::XOR) = self.current() {
            self.advance();
            let right = self.parse_bitwise_and()?;
            left = Expr::BinaryExpr {
                left: Box::new(left),
                op: BinaryOp::XOR,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_equality()?;
        while let Some(Token::AND) = self.current() {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::BinaryExpr {
                left: Box::new(left),
                op: BinaryOp::AND,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = match self.current() {
                Some(Token::DEqual)   => BinaryOp::Equal,
                Some(Token::NotEqual) => BinaryOp::NotEqual,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::BinaryExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_shift()?;
        loop {
            let op = match self.current() {
                Some(Token::Lesser)       => BinaryOp::Less,
                Some(Token::Greater)      => BinaryOp::Greater,
                Some(Token::LesserEqual)  => BinaryOp::LessEqual,
                Some(Token::GreaterEqual) => BinaryOp::GreaterEqual,
                _ => break,
            };
            self.advance();
            let right = self.parse_shift()?;
            left = Expr::BinaryExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.current() {
                Some(Token::LBS) => BinaryOp::BitShiftLeft,
                Some(Token::RBS) => BinaryOp::BitShiftRight,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::BinaryExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.current() {
                Some(Token::Plus)  => BinaryOp::Add,
                Some(Token::Minus) => BinaryOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinaryExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_exponent()?;
        loop {
            let op = match self.current() {
                Some(Token::Star)     => BinaryOp::Mul,
                Some(Token::FSlash)   => BinaryOp::Div,
                Some(Token::Squiggle) => BinaryOp::Modulo,
                Some(Token::Percent)  => BinaryOp::Percent,
                _ => break,
            };
            self.advance();
            let right = self.parse_exponent()?;
            left = Expr::BinaryExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_exponent(&mut self) -> Result<Expr, String> {
        let left = self.parse_unary()?;
        if let Some(Token::DStar) = self.current() {
            self.advance();
            let right = self.parse_exponent()?;
            return Ok(Expr::BinaryExpr {
                left: Box::new(left),
                op: BinaryOp::Exp,
                right: Box::new(right),
            });
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.current() {
            Some(Token::Not) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnaryExpr { op: UnaryOp::Not, expr: Box::new(expr) })
            }
            Some(Token::Minus) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnaryExpr { op: UnaryOp::Neg, expr: Box::new(expr) })
            }
            _ => self.parse_postfix()
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.current() {
                Some(Token::Dot) => {
                    self.advance();
                    let name = match self.current() {
                        Some(Token::Identifier(n)) => { let n = n.clone(); self.advance(); n }
                        Some(t) => return Err(format!("expected field or method name but got {:?}", t)),
                        None => return Err("expected field or method name but got EOF".to_string()),
                    };
                    if let Some(Token::LParen) = self.current() {
                        let args = self.parse_args()?;
                        expr = Expr::MethodCall {
                            object: Box::new(expr),
                            method: name,
                            args,
                        };
                    } else {
                        expr = Expr::MethodCall {
                            object: Box::new(expr),
                            method: name,
                            args: Vec::new(),
                        };
                    }
                }
                Some(Token::LSBracket) => {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(&Token::RSBracket, "parse_postfix::index::rbracket")?;
                    expr = Expr::IndexExpr {
                        object: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                Some(Token::Lesser) => {
                    // disambiguate cast expr<type> from comparison expr < expr
                    // look ahead to check if this is a cast or a comparison
                    if self.is_cast() {
                        self.advance(); // consume '<'
                        let ty = self.parse_type()?;
                        self.expect(&Token::Greater, "parse_postfix::cast::rangle")?;
                        expr = Expr::CastExpr {
                            expr: Box::new(expr),
                            to: ty,
                        };
                    } else {
                        break; // let parse_comparison handle it
                    }
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.current() {
            Some(Token::IntLiteral(n))    => { let n = *n; self.advance(); Ok(Expr::Literal(Literal::Int(n))) }
            Some(Token::FloatLiteral(n))  => { let n = *n; self.advance(); Ok(Expr::Literal(Literal::Float(n))) }
            Some(Token::DoubleLiteral(n)) => { let n = *n; self.advance(); Ok(Expr::Literal(Literal::Double(n))) }
            Some(Token::BoolLiteral(b))   => { let b = *b; self.advance(); Ok(Expr::Literal(Literal::Bool(b))) }
            Some(Token::Null)             => { self.advance(); Ok(Expr::Literal(Literal::Null)) }
            Some(Token::StringLiteral(s)) => { let s = s.clone(); self.advance(); Ok(Expr::Literal(Literal::Str(s))) }

            Some(Token::DollarSign) => {
                self.advance();
                match self.current() {
                    Some(Token::StringLiteral(s)) => {
                        let s = s.clone();
                        self.advance();
                        Ok(Expr::InterpolatedString(self.parse_interpolated(&s)?))
                    }
                    Some(t) => Err(format!("expected string after $ but got {:?}", t)),
                    None => Err("expected string after $ but got EOF".to_string()),
                }
            }

            Some(Token::Typeof) => {
                self.advance();
                self.expect(&Token::Lesser, "parse_primary::typeof::langle")?;
                let ty = self.parse_type()?;
                self.expect(&Token::Greater, "parse_primary::typeof::rangle")?;
                self.expect(&Token::LParen, "parse_primary::typeof::lparen")?;
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen, "parse_primary::typeof::rparen")?;
                Ok(Expr::TypeofExpr { ty, expr: Box::new(expr) })
            }

            Some(Token::Underscore) => {
                self.advance();
                Ok(Expr::Identifier("_".to_string()))
            }

            Some(Token::LSBracket) => {
                self.advance();
                let mut items = Vec::new();
                while let Some(t) = self.current() {
                    if matches!(t, Token::RSBracket) { break; }
                    items.push(self.parse_expr()?);
                    match self.current() {
                        Some(Token::Comma)      => { self.advance(); }
                        Some(Token::RSBracket)  => break,
                        Some(t) => return Err(format!("expected ',' or ']' but got {:?}", t)),
                        None => return Err("expected ',' or ']' but got EOF".to_string()),
                    }
                }
                self.expect(&Token::RSBracket, "parse_primary::list::rbracket")?;
                Ok(Expr::ListLiteral(items))
            }

            Some(Token::LParen) => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen, "parse_primary::grouped::rparen")?;
                Ok(expr)
            }

            Some(Token::Identifier(n)) => {
                let name = n.clone();
                self.advance();
                if let Some(Token::LParen) = self.current() {
                    let args = self.parse_args()?;
                    Ok(Expr::FunctionCall { name, args })
                } else {
                    Ok(Expr::Identifier(name))
                }
            }

            Some(t) => Err(format!("unexpected token in expression: {:?} [pos/before: {:?} / {:?}]", t, self.pos, self.tokens[self.pos - 1])),
            None => Err("unexpected EOF in expression".to_string()),
        }
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, String> {
        self.expect(&Token::LParen, "parse_args::lparen")?;
        let mut args = Vec::new();
        while let Some(t) = self.current() {
            if matches!(t, Token::RParen) { break; }
            args.push(self.parse_expr()?);
            match self.current() {
                Some(Token::Comma)  => { self.advance(); }
                Some(Token::RParen) => break,
                Some(t) => return Err(format!("expected ',' or ')' but got {:?}", t)),
                None => return Err("expected ',' or ')' but got EOF".to_string()),
            }
        }
        self.expect(&Token::RParen, "parse_args::rparen")?;
        Ok(args)
    }

    fn parse_interpolated(&self, s: &str) -> Result<Vec<StringPart>, String> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                if !current.is_empty() {
                    parts.push(StringPart::Literal(current.clone()));
                    current.clear();
                }
                let mut expr_str = String::new();
                let mut depth = 1;
                while let Some(c) = chars.next() {
                    match c {
                        '{' => { depth += 1; expr_str.push(c); }
                        '}' => {
                            depth -= 1;
                            if depth == 0 { break; }
                            expr_str.push(c);
                        }
                        _ => expr_str.push(c),
                    }
                }
                let mut inner_lexer = crate::lexer::Lexer::new(&expr_str);
                let inner_tokens = inner_lexer.tokenize();
                let mut inner_parser = Parser::new(inner_tokens);
                let expr = inner_parser.parse_expr()?;
                parts.push(StringPart::Expr(expr));
            } else {
                current.push(c);
            }
        }

        if !current.is_empty() {
            parts.push(StringPart::Literal(current));
        }

        Ok(parts)
    }

    fn is_variable_decl(&self) -> bool {
        let mut i = self.pos + 1; // skip the type identifier
        // skip past <...> if present
        if let Some(Token::Lesser) = self.tokens.get(i) {
            i += 1;
            let mut depth = 1;
            while i < self.tokens.len() {
                match self.tokens.get(i) {
                    Some(Token::Lesser) => { depth += 1; i += 1; }
                    Some(Token::Greater) => {
                        depth -= 1;
                        i += 1;
                        if depth == 0 { break; }
                    }
                    _ => { i += 1; }
                }
            }
        }
        // after type (and optional <...>), check if next token is an identifier (variable name)
        matches!(self.tokens.get(i), Some(Token::Identifier(_)))
    }

    fn is_cast(&self) -> bool {
        // peek past '<' to see if it looks like a type followed by '>'
        // types start with: int, float, double, str, bool, or an Identifier
        // e.g. x<int> x<str> x<MyType>  -- casts
        // e.g. x < 10  x < y            -- comparisons
        let i = self.pos + 1; // position after '<'
        match self.tokens.get(i) {
            Some(Token::Int)    |
            Some(Token::Float)  |
            Some(Token::Double) |
            Some(Token::Str)    |
            Some(Token::Bool)   => {
                // check token after the type is '>'
                matches!(self.tokens.get(i + 1), Some(Token::Greater))
            }
            Some(Token::Identifier(_)) => {
                // check token after the identifier is '>'
                matches!(self.tokens.get(i + 1), Some(Token::Greater))
            }
            _ => false
        }
    }
}