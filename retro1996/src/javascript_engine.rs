use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::fmt;
use encoding_rs::Encoding;
use crate::engine::TrussCore;

#[derive(Debug, Clone, PartialEq)]
pub enum JsValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Object(JsObject),
    Array(Vec<JsValue>),
    Function(JsFunction),
}

#[derive(Debug, Clone, PartialEq)]
pub struct JsObject {
    pub properties: HashMap<String, JsValue>,
    pub prototype: Option<Box<JsObject>>,
    pub class_name: String,
    pub methods: HashMap<String, JsFunction>,
}

impl JsObject {
    pub fn new(class_name: String) -> Self {
        JsObject {
            properties: HashMap::new(),
            prototype: None,
            class_name,
            methods: HashMap::new(),
        }
    }
    
    pub fn get_property(&self, name: &str) -> Option<&JsValue> {
        self.properties.get(name)
            .or_else(|| self.prototype.as_ref().and_then(|proto| proto.get_property(name)))
    }
    
    pub fn set_property(&mut self, name: String, value: JsValue) {
        self.properties.insert(name, value);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct JsFunction {
    pub name: String,
    pub parameters: Vec<String>,
    pub body: Vec<Stmt>,
    pub scope: HashMap<String, JsValue>,
}

#[derive(Debug)]
pub enum JsError {
    SyntaxError(String),
    TypeError(String),
    ReferenceError(String),
    RangeError(String),
    InternalError(String),
}

impl fmt::Display for JsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsError::SyntaxError(msg) => write!(f, "SyntaxError: {}", msg),
            JsError::TypeError(msg) => write!(f, "TypeError: {}", msg),
            JsError::ReferenceError(msg) => write!(f, "ReferenceError: {}", msg),
            JsError::RangeError(msg) => write!(f, "RangeError: {}", msg),
            JsError::InternalError(msg) => write!(f, "InternalError: {}", msg),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Identifier(String),
    Number(f64),
    String(String),
    Boolean(bool),
    Null,
    Undefined,
    Plus,
    Minus,
    Asterisk,
    Slash,
    Percent,
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
    LogicalAnd,
    LogicalOr,
    Bang,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Dot,
    Comma,
    Semicolon,
    Colon,
    Assign,
    If,
    Else,
    For,
    While,
    Do,
    Var,
    Function,
    Return,
    Break,
    Continue,
    True,
    False,
    New,
    Delete,
    Typeof,
    In,
    Instanceof,
    PlusPlus,
    MinusMinus,
    PlusAssign,
    MinusAssign,
    TimesAssign,
    DivideAssign,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    RightShift,
    UnsignedRightShift,
    Tilde,
    Question,
    Eof,
}

pub struct Lexer {
    input: Vec<u8>,
    position: usize,
    read_position: usize,
    ch: Option<u8>,
}

impl Lexer {
    pub fn new(input: &[u8]) -> Self {
        let mut lexer = Lexer {
            input: input.to_vec(),
            position: 0,
            read_position: 0,
            ch: None,
        };
        lexer.read_char();
        lexer
    }
    
    fn read_char(&mut self) {
        if self.read_position >= self.input.len() {
            self.ch = None;
        } else {
            self.ch = Some(self.input[self.read_position]);
        }
        self.position = self.read_position;
        self.read_position += 1;
    }
    
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.ch {
            if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' {
                self.read_char();
            } else {
                break;
            }
        }
    }
    
    fn read_identifier(&mut self) -> String {
        let start = self.position;
        while let Some(ch) = self.ch {
            if ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'$' {
                self.read_char();
            } else {
                break;
            }
        }
        let bytes = &self.input[start..self.position];
        String::from_utf8_lossy(bytes).into_owned()
    }
    
    fn read_number(&mut self) -> f64 {
        let start = self.position;
        while let Some(ch) = self.ch {
            if ch.is_ascii_digit() || ch == b'.' {
                self.read_char();
            } else {
                break;
            }
        }
        let bytes = &self.input[start..self.position];
        String::from_utf8_lossy(bytes).parse().unwrap_or(0.0)
    }
    
    fn read_string(&mut self) -> String {
        self.read_char();
        let start = self.position;
        while let Some(ch) = self.ch {
            if ch == b'"' || ch == b'\'' {
                break;
            }
            self.read_char();
        }
        let bytes = &self.input[start..self.position];
        let result = String::from_utf8_lossy(bytes).into_owned();
        self.read_char();
        result
    }
    
    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();
        
        let tok = match self.ch {
            Some(b'=') => {
                if self.peek_char() == Some(b'=') {
                    self.read_char();
                    self.read_char();
                    if self.peek_char() == Some(b'=') {
                        self.read_char();
                        Token::Equal
                    } else {
                        Token::Equal
                    }
                } else {
                    self.read_char();
                    Token::Assign
                }
            },
            Some(b'!') => {
                if self.peek_char() == Some(b'=') {
                    self.read_char();
                    self.read_char();
                    if self.peek_char() == Some(b'=') {
                        self.read_char();
                        Token::NotEqual
                    } else {
                        Token::NotEqual
                    }
                } else {
                    self.read_char();
                    Token::Bang
                }
            },
            Some(b'<') => {
                if self.peek_char() == Some(b'=') {
                    self.read_char();
                    self.read_char();
                    Token::LessThanOrEqual
                } else if self.peek_char() == Some(b'<') {
                    self.read_char();
                    self.read_char();
                    if self.peek_char() == Some(b'=') {
                        self.read_char();
                        Token::LeftShift
                    } else {
                        Token::LeftShift
                    }
                } else {
                    self.read_char();
                    Token::LessThan
                }
            },
            Some(b'>') => {
                if self.peek_char() == Some(b'=') {
                    self.read_char();
                    self.read_char();
                    Token::GreaterThanOrEqual
                } else if self.peek_char() == Some(b'>') {
                    self.read_char();
                    self.read_char();
                    if self.peek_char() == Some(b'>') {
                        self.read_char();
                        if self.peek_char() == Some(b'=') {
                            self.read_char();
                            Token::UnsignedRightShift
                        } else {
                            Token::UnsignedRightShift
                        }
                    } else {
                        Token::RightShift
                    }
                } else {
                    self.read_char();
                    Token::GreaterThan
                }
            },
            Some(b'&') => {
                if self.peek_char() == Some(b'&') {
                    self.read_char();
                    self.read_char();
                    Token::LogicalAnd
                } else {
                    self.read_char();
                    Token::BitwiseAnd
                }
            },
            Some(b'|') => {
                if self.peek_char() == Some(b'|') {
                    self.read_char();
                    self.read_char();
                    Token::LogicalOr
                } else {
                    self.read_char();
                    Token::BitwiseOr
                }
            },
            Some(b'^') => {
                self.read_char();
                Token::BitwiseXor
            },
            Some(b'~') => {
                self.read_char();
                Token::Tilde
            },
            Some(b'?') => {
                self.read_char();
                Token::Question
            },
            Some(b'+') => {
                if self.peek_char() == Some(b'+') {
                    self.read_char();
                    self.read_char();
                    Token::PlusPlus
                } else if self.peek_char() == Some(b'=') {
                    self.read_char();
                    self.read_char();
                    Token::PlusAssign
                } else {
                    self.read_char();
                    Token::Plus
                }
            },
            Some(b'-') => {
                if self.peek_char() == Some(b'-') {
                    self.read_char();
                    self.read_char();
                    Token::MinusMinus
                } else if self.peek_char() == Some(b'=') {
                    self.read_char();
                    self.read_char();
                    Token::MinusAssign
                } else {
                    self.read_char();
                    Token::Minus
                }
            },
            Some(b'*') => {
                if self.peek_char() == Some(b'=') {
                    self.read_char();
                    self.read_char();
                    Token::TimesAssign
                } else {
                    self.read_char();
                    Token::Asterisk
                }
            },
            Some(b'/') => {
                if self.peek_char() == Some(b'=') {
                    self.read_char();
                    self.read_char();
                    Token::DivideAssign
                } else {
                    self.read_char();
                    Token::Slash
                }
            },
            Some(b'%') => {
                if self.peek_char() == Some(b'=') {
                    self.read_char();
                    self.read_char();
                    Token::Percent
                } else {
                    self.read_char();
                    Token::Percent
                }
            },
            Some(b'(') => { self.read_char(); Token::LeftParen },
            Some(b')') => { self.read_char(); Token::RightParen },
            Some(b'{') => { self.read_char(); Token::LeftBrace },
            Some(b'}') => { self.read_char(); Token::RightBrace },
            Some(b'[') => { self.read_char(); Token::LeftBracket },
            Some(b']') => { self.read_char(); Token::RightBracket },
            Some(b'.') => { self.read_char(); Token::Dot },
            Some(b',') => { self.read_char(); Token::Comma },
            Some(b';') => { self.read_char(); Token::Semicolon },
            Some(b':') => { self.read_char(); Token::Colon },
            Some(b'"') | Some(b'\'') => Token::String(self.read_string()),
            Some(ch) if ch.is_ascii_digit() => Token::Number(self.read_number()),
            Some(ch) if ch.is_ascii_alphabetic() || ch == b'_' || ch == b'$' => {
                let ident = self.read_identifier();
                match ident.as_str() {
                    "if" => Token::If,
                    "else" => Token::Else,
                    "for" => Token::For,
                    "while" => Token::While,
                    "do" => Token::Do,
                    "var" => Token::Var,
                    "function" => Token::Function,
                    "return" => Token::Return,
                    "break" => Token::Break,
                    "continue" => Token::Continue,
                    "true" => Token::True,
                    "false" => Token::False,
                    "null" => Token::Null,
                    "undefined" => Token::Undefined,
                    "new" => Token::New,
                    "delete" => Token::Delete,
                    "typeof" => Token::Typeof,
                    "in" => Token::In,
                    "instanceof" => Token::Instanceof,
                    _ => Token::Identifier(ident),
                }
            },
            None => Token::Eof,
            _ => {
                self.read_char();
                Token::Eof
            }
        };
        
        tok
    }
    
    fn peek_char(&self) -> Option<u8> {
        if self.read_position >= self.input.len() {
            None
        } else {
            Some(self.input[self.read_position])
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Identifier(String),
    Number(f64),
    String(String),
    Boolean(bool),
    Null,
    Undefined,
    BinaryOp(Box<Expr>, Token, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Member(Box<Expr>, String),
    Array(Vec<Expr>),
    Object(HashMap<String, Expr>),
    FunctionDef(String, Vec<String>, Vec<Stmt>),
    Index(Box<Expr>, Box<Expr>),
    New(Box<Expr>, Vec<Expr>),
    Typeof(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expression(Expr),
    VariableDecl(String, Option<Expr>),
    FunctionDecl(String, Vec<String>, Box<[Stmt]>),
    If(Expr, Box<[Stmt]>, Option<Box<[Stmt]>>),
    For(Option<Box<Stmt>>, Option<Expr>, Option<Expr>, Box<[Stmt]>),
    While(Expr, Box<[Stmt]>),
    Return(Option<Expr>),
    Block(Box<[Stmt]>),
    ForIn(String, Expr, Box<[Stmt]>),
}

pub struct Parser {
    lexer: Lexer,
    current_token: Token,
    next_token: Token,
}

impl Parser {
    pub fn new(input: &[u8]) -> Self {
        let mut lexer = Lexer::new(input);
        let current_token = lexer.next_token();
        let next_token = lexer.next_token();
        
        Parser {
            lexer,
            current_token,
            next_token,
        }
    }
    
    fn advance(&mut self) {
        self.current_token = self.next_token.clone();
        self.next_token = self.lexer.next_token();
    }
    
    fn expect_token(&mut self, expected: Token) -> Result<(), JsError> {
        if self.current_token == expected {
            self.advance();
            Ok(())
        } else {
            Err(JsError::SyntaxError(format!("Expected {:?}, got {:?}", expected, self.current_token)))
        }
    }
    
    pub fn parse_program(&mut self) -> Result<Vec<Stmt>, JsError> {
        let mut statements = Vec::new();
        
        while self.current_token != Token::Eof {
            statements.push(self.parse_statement()?);
        }
        
        Ok(statements)
    }
    
    fn parse_statement(&mut self) -> Result<Stmt, JsError> {
        match self.current_token {
            Token::Var => self.parse_variable_declaration(),
            Token::Function => self.parse_function_declaration(),
            Token::If => self.parse_if_statement(),
            Token::For => self.parse_for_statement(),
            Token::While => self.parse_while_statement(),
            Token::Return => self.parse_return_statement(),
            Token::LeftBrace => self.parse_block_statement(),
            _ => Ok(Stmt::Expression(self.parse_expression()?)),
        }
    }
    
    fn parse_variable_declaration(&mut self) -> Result<Stmt, JsError> {
        self.advance();
        let ident = if let Token::Identifier(name) = self.current_token.clone() {
            name
        } else {
            return Err(JsError::SyntaxError("Expected identifier after 'var'".to_string()));
        };
        
        self.advance();
        
        let mut initializer = None;
        if self.current_token == Token::Assign {
            self.advance();
            initializer = Some(self.parse_expression()?);
        }
        
        self.expect_token(Token::Semicolon)?;
        
        Ok(Stmt::VariableDecl(ident, initializer))
    }
    
    fn parse_function_declaration(&mut self) -> Result<Stmt, JsError> {
        self.advance();
        let name = if let Token::Identifier(n) = self.current_token.clone() {
            n
        } else {
            return Err(JsError::SyntaxError("Expected function name".to_string()));
        };
        
        self.advance();
        self.expect_token(Token::LeftParen)?;
        
        let mut params = Vec::new();
        if self.current_token != Token::RightParen {
            loop {
                if let Token::Identifier(param) = self.current_token.clone() {
                    params.push(param);
                    self.advance();
                } else {
                    return Err(JsError::SyntaxError("Expected parameter name".to_string()));
                }
                
                if self.current_token == Token::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        
        self.expect_token(Token::RightParen)?;
        self.expect_token(Token::LeftBrace)?;
        
        let body = self.parse_block_body()?.into_boxed_slice();
        
        Ok(Stmt::FunctionDecl(name, params, body))
    }
    
    fn parse_if_statement(&mut self) -> Result<Stmt, JsError> {
        self.advance();
        self.expect_token(Token::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect_token(Token::RightParen)?;
        
        let consequence = self.parse_block_body()?.into_boxed_slice();
        let alternative = if self.current_token == Token::Else {
            self.advance();
            Some(self.parse_block_body()?.into_boxed_slice())
        } else {
            None
        };
        
        Ok(Stmt::If(condition, consequence, alternative))
    }
    
    fn parse_for_statement(&mut self) -> Result<Stmt, JsError> {
        self.advance();
        self.expect_token(Token::LeftParen)?;
        
        let init = if self.current_token != Token::Semicolon {
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };
        
        self.expect_token(Token::Semicolon)?;
        
        let condition = if self.current_token != Token::Semicolon {
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        self.expect_token(Token::Semicolon)?;
        
        let increment = if self.current_token != Token::RightParen {
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        self.expect_token(Token::RightParen)?;
        
        let body = self.parse_block_body()?.into_boxed_slice();
        
        Ok(Stmt::For(init, condition, increment, body))
    }
    
    fn parse_while_statement(&mut self) -> Result<Stmt, JsError> {
        self.advance();
        self.expect_token(Token::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect_token(Token::RightParen)?;
        
        let body = self.parse_block_body()?.into_boxed_slice();
        
        Ok(Stmt::While(condition, body))
    }
    
    fn parse_return_statement(&mut self) -> Result<Stmt, JsError> {
        self.advance();
        
        let expr = if self.current_token != Token::Semicolon && self.current_token != Token::RightBrace {
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        self.expect_token(Token::Semicolon)?;
        
        Ok(Stmt::Return(expr))
    }
    
    fn parse_block_statement(&mut self) -> Result<Stmt, JsError> {
        self.expect_token(Token::LeftBrace)?;
        let statements = self.parse_block_body()?.into_boxed_slice();
        self.expect_token(Token::RightBrace)?;
        
        Ok(Stmt::Block(statements))
    }
    
    fn parse_block_body(&mut self) -> Result<Vec<Stmt>, JsError> {
        let mut statements = Vec::new();
        
        while self.current_token != Token::RightBrace && self.current_token != Token::Eof {
            statements.push(self.parse_statement()?);
        }
        
        Ok(statements)
    }
    
    fn parse_expression(&mut self) -> Result<Expr, JsError> {
        self.parse_assignment()
    }
    
    fn parse_assignment(&mut self) -> Result<Expr, JsError> {
        let left = self.parse_logical_or()?;
        
        if matches!(self.current_token, Token::Assign | Token::PlusAssign | Token::MinusAssign | Token::TimesAssign | Token::DivideAssign) {
            let op = self.current_token.clone();
            self.advance();
            let right = self.parse_assignment()?;
            Ok(Expr::BinaryOp(Box::new(left), op, Box::new(right)))
        } else {
            Ok(left)
        }
    }
    
    fn parse_logical_or(&mut self) -> Result<Expr, JsError> {
        let mut left = self.parse_logical_and()?;
        
        while self.current_token == Token::LogicalOr {
            let op = self.current_token.clone();
            self.advance();
            let right = self.parse_logical_and()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }
        
        Ok(left)
    }
    
    fn parse_logical_and(&mut self) -> Result<Expr, JsError> {
        let mut left = self.parse_equality()?;
        
        while self.current_token == Token::LogicalAnd {
            let op = self.current_token.clone();
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }
        
        Ok(left)
    }
    
    fn parse_equality(&mut self) -> Result<Expr, JsError> {
        let mut left = self.parse_relational()?;
        
        while self.current_token == Token::Equal || self.current_token == Token::NotEqual {
            let op = self.current_token.clone();
            self.advance();
            let right = self.parse_relational()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }
        
        Ok(left)
    }
    
    fn parse_relational(&mut self) -> Result<Expr, JsError> {
        let mut left = self.parse_shift()?;
        
        while matches!(self.current_token, Token::LessThan | Token::LessThanOrEqual | Token::GreaterThan | Token::GreaterThanOrEqual) {
            let op = self.current_token.clone();
            self.advance();
            let right = self.parse_shift()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }
        
        Ok(left)
    }
    
    fn parse_shift(&mut self) -> Result<Expr, JsError> {
        let mut left = self.parse_addition()?;
        
        while matches!(self.current_token, Token::LeftShift | Token::RightShift | Token::UnsignedRightShift) {
            let op = self.current_token.clone();
            self.advance();
            let right = self.parse_addition()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }
        
        Ok(left)
    }
    
    fn parse_addition(&mut self) -> Result<Expr, JsError> {
        let mut left = self.parse_multiplication()?;
        
        while self.current_token == Token::Plus || self.current_token == Token::Minus {
            let op = self.current_token.clone();
            self.advance();
            let right = self.parse_multiplication()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }
        
        Ok(left)
    }
    
    fn parse_multiplication(&mut self) -> Result<Expr, JsError> {
        let mut left = self.parse_unary()?;
        
        while self.current_token == Token::Asterisk || 
              self.current_token == Token::Slash || 
              self.current_token == Token::Percent {
            let op = self.current_token.clone();
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }
        
        Ok(left)
    }
    
    fn parse_unary(&mut self) -> Result<Expr, JsError> {
        match self.current_token {
            Token::Typeof => {
                self.advance();
                let right = self.parse_unary()?;
                Ok(Expr::Typeof(Box::new(right)))
            },
            _ => self.parse_call(),
        }
    }
    
    fn parse_call(&mut self) -> Result<Expr, JsError> {
        let mut expr = self.parse_primary()?;
        
        loop {
            if self.current_token == Token::LeftParen {
                expr = self.parse_call_expr(expr)?;
            } else if self.current_token == Token::LeftBracket {
                expr = self.parse_index_expr(expr)?;
            } else if self.current_token == Token::Dot {
                self.advance();
                if let Token::Identifier(prop) = self.current_token.clone() {
                    self.advance();
                    expr = Expr::Member(Box::new(expr), prop);
                } else {
                    return Err(JsError::SyntaxError("Expected property name after '.'".to_string()));
                }
            } else {
                break;
            }
        }
        
        Ok(expr)
    }
    
    fn parse_call_expr(&mut self, callee: Expr) -> Result<Expr, JsError> {
        self.advance();
        
        let mut args = Vec::new();
        if self.current_token != Token::RightParen {
            loop {
                args.push(self.parse_expression()?);
                
                if self.current_token == Token::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        
        self.expect_token(Token::RightParen)?;
        
        Ok(Expr::Call(Box::new(callee), args))
    }
    
    fn parse_index_expr(&mut self, obj: Expr) -> Result<Expr, JsError> {
        self.advance();
        
        let index = self.parse_expression()?;
        
        self.expect_token(Token::RightBracket)?;
        
        Ok(Expr::Index(Box::new(obj), Box::new(index)))
    }
    
    fn parse_primary(&mut self) -> Result<Expr, JsError> {
        match self.current_token.clone() {
            Token::Identifier(name) => {
                self.advance();
                if name == "new" {
                    let constructor = self.parse_primary()?;
                    let args = if self.current_token == Token::LeftParen {
                        let call_expr = self.parse_call_expr(constructor.clone())?;
                        if let Expr::Call(_, args) = call_expr {
                            args
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    };
                    Ok(Expr::New(Box::new(constructor), args))
                } else {
                    Ok(Expr::Identifier(name))
                }
            },
            Token::Number(n) => {
                self.advance();
                Ok(Expr::Number(n))
            },
            Token::String(s) => {
                self.advance();
                Ok(Expr::String(s))
            },
            Token::True => {
                self.advance();
                Ok(Expr::Boolean(true))
            },
            Token::False => {
                self.advance();
                Ok(Expr::Boolean(false))
            },
            Token::Null => {
                self.advance();
                Ok(Expr::Null)
            },
            Token::Undefined => {
                self.advance();
                Ok(Expr::Undefined)
            },
            Token::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect_token(Token::RightParen)?;
                Ok(expr)
            },
            Token::LeftBracket => {
                self.advance();
                let mut elements = Vec::new();
                
                if self.current_token != Token::RightBracket {
                    loop {
                        elements.push(self.parse_expression()?);
                        
                        if self.current_token == Token::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                
                self.expect_token(Token::RightBracket)?;
                Ok(Expr::Array(elements))
            },
            Token::LeftBrace => {
                self.advance();
                let mut props = HashMap::new();
                
                if self.current_token != Token::RightBrace {
                    loop {
                        if let Token::Identifier(key) = self.current_token.clone() {
                            self.advance();
                            self.expect_token(Token::Colon)?;
                            let value = self.parse_expression()?;
                            props.insert(key, value);
                        } else {
                            return Err(JsError::SyntaxError("Expected property name".to_string()));
                        }
                        
                        if self.current_token == Token::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                
                self.expect_token(Token::RightBrace)?;
                Ok(Expr::Object(props))
            },
            Token::Function => {
                self.advance();
                let name = if let Token::Identifier(n) = self.current_token.clone() {
                    self.advance();
                    n
                } else {
                    String::new()
                };
                
                self.expect_token(Token::LeftParen)?;
                
                let mut params = Vec::new();
                if self.current_token != Token::RightParen {
                    loop {
                        if let Token::Identifier(param) = self.current_token.clone() {
                            params.push(param);
                            self.advance();
                        } else {
                            return Err(JsError::SyntaxError("Expected parameter name".to_string()));
                        }
                        
                        if self.current_token == Token::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                
                self.expect_token(Token::RightParen)?;
                self.expect_token(Token::LeftBrace)?;
                
                let body = self.parse_block_body()?;
                
                Ok(Expr::FunctionDef(name, params, body))
            },
            _ => Err(JsError::SyntaxError(format!("Unexpected token: {:?}", self.current_token))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub variables: HashMap<String, JsValue>,
    pub parent: Option<Box<Scope>>,
}

impl Scope {
    pub fn new(parent: Option<Box<Scope>>) -> Self {
        Scope {
            variables: HashMap::new(),
            parent,
        }
    }
    
    pub fn get(&self, name: &str) -> Option<&JsValue> {
        if let Some(value) = self.variables.get(name) {
            Some(value)
        } else if let Some(ref parent) = self.parent {
            parent.get(name)
        } else {
            None
        }
    }
    
    pub fn set(&mut self, name: String, value: JsValue) {
        self.variables.insert(name, value);
    }
    
    pub fn define(&mut self, name: String, value: JsValue) {
        self.variables.insert(name, value);
    }
}

#[derive(Debug)]
pub struct ExecutionContext {
    pub scope: Scope,
    pub this_binding: JsValue,
    pub return_flag: bool,
}

impl ExecutionContext {
    pub fn new(scope: Scope, this_binding: JsValue) -> Self {
        ExecutionContext {
            scope,
            this_binding,
            return_flag: false,
        }
    }
}

pub struct ChronoScript {
    pub global_scope: HashMap<String, JsValue>,
    pub built_in_functions: HashMap<String, BuiltInFunction>,
    pub objects: HashMap<String, JsObject>,
    pub prototypes: HashMap<String, JsObject>,
    pub event_listeners: HashMap<String, JsFunction>,
    pub error_buffer: Vec<String>,
    pub output_buffer: String,
    pub trusscore_bridge: Option<Arc<Mutex<TrussCore>>>,
    pub execution_contexts: Vec<ExecutionContext>,
    pub parsing_in_progress: bool,
    pub parse_insertion_point: Option<usize>,
    pub source_encoding: &'static Encoding,
    pub default_encoding: &'static Encoding,
}

impl ChronoScript {
    pub fn new() -> Self {
        let mut engine = ChronoScript {
            global_scope: HashMap::new(),
            built_in_functions: HashMap::new(),
            objects: HashMap::new(),
            prototypes: HashMap::new(),
            event_listeners: HashMap::new(),
            error_buffer: Vec::new(),
            output_buffer: String::new(),
            trusscore_bridge: None,
            execution_contexts: Vec::new(),
            parsing_in_progress: false,
            parse_insertion_point: None,
            source_encoding: encoding_rs::WINDOWS_1252,
            default_encoding: encoding_rs::WINDOWS_1252,
        };
        
        engine.init_built_ins();
        engine.init_prototypes();
        engine.init_global_objects();
        engine
    }
    
    fn init_built_ins(&mut self) {
        self.built_in_functions.insert("alert".to_string(), BuiltInFunction::Native(alert));
        self.built_in_functions.insert("confirm".to_string(), BuiltInFunction::Native(confirm));
        self.built_in_functions.insert("prompt".to_string(), BuiltInFunction::Native(prompt));
        self.built_in_functions.insert("parseInt".to_string(), BuiltInFunction::Native(parse_int));
        self.built_in_functions.insert("parseFloat".to_string(), BuiltInFunction::Native(parse_float));
        self.built_in_functions.insert("isNaN".to_string(), BuiltInFunction::Native(is_nan));
        self.built_in_functions.insert("isFinite".to_string(), BuiltInFunction::Native(is_finite));
        self.built_in_functions.insert("eval".to_string(), BuiltInFunction::Native(eval));
        self.built_in_functions.insert("escape".to_string(), BuiltInFunction::Native(escape));
        self.built_in_functions.insert("unescape".to_string(), BuiltInFunction::Native(unescape));
        self.built_in_functions.insert("String.fromCharCode".to_string(), BuiltInFunction::Native(string_from_char_code));
        
        let mut math_obj = JsObject::new("Math".to_string());
        math_obj.properties.insert("PI".to_string(), JsValue::Number(std::f64::consts::PI));
        math_obj.properties.insert("E".to_string(), JsValue::Number(std::f64::consts::E));
        math_obj.methods.insert("abs".to_string(), JsFunction {
            name: "abs".to_string(),
            parameters: vec!["x".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        math_obj.methods.insert("ceil".to_string(), JsFunction {
            name: "ceil".to_string(),
            parameters: vec!["x".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        math_obj.methods.insert("floor".to_string(), JsFunction {
            name: "floor".to_string(),
            parameters: vec!["x".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        math_obj.methods.insert("round".to_string(), JsFunction {
            name: "round".to_string(),
            parameters: vec!["x".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        math_obj.methods.insert("max".to_string(), JsFunction {
            name: "max".to_string(),
            parameters: vec!["a".to_string(), "b".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        math_obj.methods.insert("min".to_string(), JsFunction {
            name: "min".to_string(),
            parameters: vec!["a".to_string(), "b".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        math_obj.methods.insert("random".to_string(), JsFunction {
            name: "random".to_string(),
            parameters: vec![],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        self.objects.insert("Math".to_string(), math_obj);
        
        let mut date_obj = JsObject::new("Date".to_string());
        date_obj.methods.insert("UTC".to_string(), JsFunction {
            name: "UTC".to_string(),
            parameters: vec![
                "year".to_string(), "month".to_string(), "day".to_string(),
                "hours".to_string(), "minutes".to_string(), "seconds".to_string(), "ms".to_string()
            ],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        self.objects.insert("Date".to_string(), date_obj);
        
        let mut regexp_obj = JsObject::new("RegExp".to_string());
        self.objects.insert("RegExp".to_string(), regexp_obj);
    }
    
    fn init_prototypes(&mut self) {
        let mut string_proto = JsObject::new("String.prototype".to_string());
        string_proto.methods.insert("charAt".to_string(), JsFunction {
            name: "charAt".to_string(),
            parameters: vec!["index".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        string_proto.methods.insert("charCodeAt".to_string(), JsFunction {
            name: "charCodeAt".to_string(),
            parameters: vec!["index".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        string_proto.methods.insert("concat".to_string(), JsFunction {
            name: "concat".to_string(),
            parameters: vec!["other".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        string_proto.methods.insert("indexOf".to_string(), JsFunction {
            name: "indexOf".to_string(),
            parameters: vec!["searchValue".to_string(), "fromIndex".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        string_proto.methods.insert("lastIndexOf".to_string(), JsFunction {
            name: "lastIndexOf".to_string(),
            parameters: vec!["searchValue".to_string(), "fromIndex".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        string_proto.methods.insert("slice".to_string(), JsFunction {
            name: "slice".to_string(),
            parameters: vec!["start".to_string(), "end".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        string_proto.methods.insert("substring".to_string(), JsFunction {
            name: "substring".to_string(),
            parameters: vec!["start".to_string(), "end".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        string_proto.methods.insert("toLowerCase".to_string(), JsFunction {
            name: "toLowerCase".to_string(),
            parameters: vec![],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        string_proto.methods.insert("toUpperCase".to_string(), JsFunction {
            name: "toUpperCase".to_string(),
            parameters: vec![],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        string_proto.methods.insert("trim".to_string(), JsFunction {
            name: "trim".to_string(),
            parameters: vec![],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        string_proto.methods.insert("split".to_string(), JsFunction {
            name: "split".to_string(),
            parameters: vec!["separator".to_string(), "limit".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        string_proto.methods.insert("substr".to_string(), JsFunction {
            name: "substr".to_string(),
            parameters: vec!["start".to_string(), "length".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        self.prototypes.insert("String".to_string(), string_proto);
        
        let mut array_proto = JsObject::new("Array.prototype".to_string());
        array_proto.methods.insert("pop".to_string(), JsFunction {
            name: "pop".to_string(),
            parameters: vec![],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        array_proto.methods.insert("push".to_string(), JsFunction {
            name: "push".to_string(),
            parameters: vec!["item".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        array_proto.methods.insert("shift".to_string(), JsFunction {
            name: "shift".to_string(),
            parameters: vec![],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        array_proto.methods.insert("unshift".to_string(), JsFunction {
            name: "unshift".to_string(),
            parameters: vec!["item".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        array_proto.methods.insert("concat".to_string(), JsFunction {
            name: "concat".to_string(),
            parameters: vec!["other".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        array_proto.methods.insert("join".to_string(), JsFunction {
            name: "join".to_string(),
            parameters: vec!["separator".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        array_proto.methods.insert("reverse".to_string(), JsFunction {
            name: "reverse".to_string(),
            parameters: vec![],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        array_proto.methods.insert("sort".to_string(), JsFunction {
            name: "sort".to_string(),
            parameters: vec!["compareFn".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        array_proto.methods.insert("slice".to_string(), JsFunction {
            name: "slice".to_string(),
            parameters: vec!["start".to_string(), "end".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        array_proto.methods.insert("splice".to_string(), JsFunction {
            name: "splice".to_string(),
            parameters: vec!["start".to_string(), "deleteCount".to_string(), "items".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        self.prototypes.insert("Array".to_string(), array_proto);
        
        let mut object_proto = JsObject::new("Object.prototype".to_string());
        self.prototypes.insert("Object".to_string(), object_proto);
    }
    
    fn init_global_objects(&mut self) {
        let window = Window::new();
        self.global_scope.insert("window".to_string(), JsValue::Object(self.window_to_js_object(window)));
        
        self.global_scope.insert("document".to_string(), JsValue::Object(self.document_to_js_object()));
        self.global_scope.insert("location".to_string(), JsValue::Object(self.location_to_js_object()));
        self.global_scope.insert("history".to_string(), JsValue::Object(self.history_to_js_object()));
        self.global_scope.insert("navigator".to_string(), JsValue::Object(self.navigator_to_js_object()));
        self.global_scope.insert("screen".to_string(), JsValue::Object(self.screen_to_js_object()));
    }
    
    fn window_to_js_object(&self, window: Window) -> JsObject {
        let mut obj = JsObject::new("Window".to_string());
        obj.properties.insert("document".to_string(), JsValue::Object(self.document_to_js_object()));
        obj.properties.insert("location".to_string(), JsValue::Object(self.location_to_js_object()));
        obj.properties.insert("history".to_string(), JsValue::Object(self.history_to_js_object()));
        obj.properties.insert("navigator".to_string(), JsValue::Object(self.navigator_to_js_object()));
        obj.properties.insert("screen".to_string(), JsValue::Object(self.screen_to_js_object()));
        obj.properties.insert("name".to_string(), JsValue::String(window.name));
        obj.properties.insert("status".to_string(), JsValue::String(window.status));
        obj.properties.insert("defaultStatus".to_string(), JsValue::String(window.default_status));
        obj.properties.insert("closed".to_string(), JsValue::Boolean(window.closed));
        
        obj.methods.insert("alert".to_string(), JsFunction {
            name: "alert".to_string(),
            parameters: vec!["message".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        obj.methods.insert("confirm".to_string(), JsFunction {
            name: "confirm".to_string(),
            parameters: vec!["message".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        obj.methods.insert("prompt".to_string(), JsFunction {
            name: "prompt".to_string(),
            parameters: vec!["message".to_string(), "default".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        obj.methods.insert("close".to_string(), JsFunction {
            name: "close".to_string(),
            parameters: vec![],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        obj.methods.insert("focus".to_string(), JsFunction {
            name: "focus".to_string(),
            parameters: vec![],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        obj.methods.insert("blur".to_string(), JsFunction {
            name: "blur".to_string(),
            parameters: vec![],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        obj.methods.insert("scrollTo".to_string(), JsFunction {
            name: "scrollTo".to_string(),
            parameters: vec!["x".to_string(), "y".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        obj.methods.insert("scrollBy".to_string(), JsFunction {
            name: "scrollBy".to_string(),
            parameters: vec!["x".to_string(), "y".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        
        obj
    }
    
    fn document_to_js_object(&self) -> JsObject {
        let mut obj = JsObject::new("Document".to_string());
        obj.properties.insert("title".to_string(), JsValue::String(self.global_scope.get("window")
            .and_then(|v| if let JsValue::Object(o) = v { o.properties.get("document").cloned() } else { None })
            .and_then(|v| if let JsValue::Object(o) = v { o.properties.get("title").cloned() } else { None })
            .and_then(|v| if let JsValue::String(s) = v { Some(s) } else { None })
            .unwrap_or_default()));
        obj.properties.insert("URL".to_string(), JsValue::String(String::new()));
        obj.properties.insert("domain".to_string(), JsValue::String(String::new()));
        obj.properties.insert("referrer".to_string(), JsValue::String(String::new()));
        obj.properties.insert("cookie".to_string(), JsValue::String(String::new()));
        obj.properties.insert("bgColor".to_string(), JsValue::String(String::new()));
        obj.properties.insert("fgColor".to_string(), JsValue::String(String::new()));
        obj.properties.insert("linkColor".to_string(), JsValue::String(String::new()));
        obj.properties.insert("vlinkColor".to_string(), JsValue::String(String::new()));
        obj.properties.insert("alinkColor".to_string(), JsValue::String(String::new()));
        
        obj.properties.insert("forms".to_string(), JsValue::Array(Vec::new()));
        obj.properties.insert("images".to_string(), JsValue::Array(Vec::new()));
        obj.properties.insert("links".to_string(), JsValue::Array(Vec::new()));
        obj.properties.insert("anchors".to_string(), JsValue::Array(Vec::new()));
        obj.properties.insert("applets".to_string(), JsValue::Array(Vec::new()));
        obj.properties.insert("embeds".to_string(), JsValue::Array(Vec::new()));
        
        obj.methods.insert("write".to_string(), JsFunction {
            name: "write".to_string(),
            parameters: vec!["text".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        obj.methods.insert("writeln".to_string(), JsFunction {
            name: "writeln".to_string(),
            parameters: vec!["text".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        obj.methods.insert("getElementById".to_string(), JsFunction {
            name: "getElementById".to_string(),
            parameters: vec!["id".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        obj.methods.insert("getElementsByName".to_string(), JsFunction {
            name: "getElementsByName".to_string(),
            parameters: vec!["name".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        obj.methods.insert("getElementsByTagName".to_string(), JsFunction {
            name: "getElementsByTagName".to_string(),
            parameters: vec!["tagname".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        obj.methods.insert("createElement".to_string(), JsFunction {
            name: "createElement".to_string(),
            parameters: vec!["tagName".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        obj.methods.insert("createTextNode".to_string(), JsFunction {
            name: "createTextNode".to_string(),
            parameters: vec!["data".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        
        obj
    }
    
    fn location_to_js_object(&self) -> JsObject {
        let mut obj = JsObject::new("Location".to_string());
        obj.properties.insert("href".to_string(), JsValue::String(String::new()));
        obj.properties.insert("protocol".to_string(), JsValue::String(String::new()));
        obj.properties.insert("host".to_string(), JsValue::String(String::new()));
        obj.properties.insert("hostname".to_string(), JsValue::String(String::new()));
        obj.properties.insert("port".to_string(), JsValue::String(String::new()));
        obj.properties.insert("pathname".to_string(), JsValue::String(String::new()));
        obj.properties.insert("search".to_string(), JsValue::String(String::new()));
        obj.properties.insert("hash".to_string(), JsValue::String(String::new()));
        
        obj.methods.insert("assign".to_string(), JsFunction {
            name: "assign".to_string(),
            parameters: vec!["url".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        obj.methods.insert("reload".to_string(), JsFunction {
            name: "reload".to_string(),
            parameters: vec![],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        obj.methods.insert("replace".to_string(), JsFunction {
            name: "replace".to_string(),
            parameters: vec!["url".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        
        obj
    }
    
    fn history_to_js_object(&self) -> JsObject {
        let mut obj = JsObject::new("History".to_string());
        obj.properties.insert("length".to_string(), JsValue::Number(0.0));
        
        obj.methods.insert("back".to_string(), JsFunction {
            name: "back".to_string(),
            parameters: vec![],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        obj.methods.insert("forward".to_string(), JsFunction {
            name: "forward".to_string(),
            parameters: vec![],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        obj.methods.insert("go".to_string(), JsFunction {
            name: "go".to_string(),
            parameters: vec!["delta".to_string()],
            body: Vec::new(),
            scope: HashMap::new(),
        });
        
        obj
    }
    
    fn navigator_to_js_object(&self) -> JsObject {
        let mut obj = JsObject::new("Navigator".to_string());
        obj.properties.insert("appCodeName".to_string(), JsValue::String("Mozilla".to_string()));
        obj.properties.insert("appName".to_string(), JsValue::String("Netscape".to_string()));
        obj.properties.insert("appVersion".to_string(), JsValue::String("3.0 (Win95)".to_string()));
        obj.properties.insert("userAgent".to_string(), JsValue::String("Mozilla/3.0 (compatible; Retro1996/3.0; Win95)".to_string()));
        obj.properties.insert("platform".to_string(), JsValue::String("Win95".to_string()));
        obj.properties.insert("onLine".to_string(), JsValue::Boolean(true));
        obj.properties.insert("cookieEnabled".to_string(), JsValue::Boolean(true));
        obj.properties.insert("javaEnabled".to_string(), JsValue::Boolean(false));
        
        obj
    }
    
    fn screen_to_js_object(&self) -> JsObject {
        let mut obj = JsObject::new("Screen".to_string());
        obj.properties.insert("width".to_string(), JsValue::Number(800.0));
        obj.properties.insert("height".to_string(), JsValue::Number(600.0));
        obj.properties.insert("availWidth".to_string(), JsValue::Number(800.0));
        obj.properties.insert("availHeight".to_string(), JsValue::Number(600.0));
        obj.properties.insert("colorDepth".to_string(), JsValue::Number(16.0));
        obj.properties.insert("pixelDepth".to_string(), JsValue::Number(16.0));
        
        obj
    }
    
    pub fn set_trusscore_bridge(&mut self, bridge: Arc<Mutex<TrussCore>>) {
        self.trusscore_bridge = Some(bridge);
    }
    
    pub fn set_encoding(&mut self, encoding: &'static Encoding) {
        self.source_encoding = encoding;
    }
    
    pub fn execute(&mut self, code: &[u8]) -> Result<JsValue, JsError> {
        let (decoded, _, _) = self.source_encoding.decode(code);
        let mut parser = Parser::new(decoded.as_bytes());
        let program = parser.parse_program()?;
        
        let global_scope = Scope::new(None);
        let mut ctx = ExecutionContext::new(global_scope, JsValue::Undefined);
        
        for stmt in program {
            let result = self.eval_stmt(stmt, &mut ctx)?;
            if ctx.return_flag {
                return Ok(result.unwrap_or(JsValue::Undefined));
            }
        }
        
        Ok(JsValue::Undefined)
    }
    
    fn eval_stmt_block(&mut self, statements: &[Stmt], ctx: &mut ExecutionContext) -> Result<Option<JsValue>, JsError> {
        for stmt in statements {
            let result = self.eval_stmt(stmt.clone(), ctx)?;
            if ctx.return_flag {
                return Ok(result);
            }
        }
        Ok(None)
    }
    
    fn eval_stmt(&mut self, stmt: Stmt, ctx: &mut ExecutionContext) -> Result<Option<JsValue>, JsError> {
        match stmt {
            Stmt::Expression(expr) => {
                let value = self.eval_expr(expr, ctx)?;
                Ok(Some(value))
            },
            Stmt::VariableDecl(name, initializer) => {
                let value = if let Some(init) = initializer {
                    self.eval_expr(init, ctx)?
                } else {
                    JsValue::Undefined
                };
                
                ctx.scope.define(name, value);
                Ok(None)
            },
            Stmt::FunctionDecl(name, params, body) => {
                let func = JsValue::Function(JsFunction {
                    name: name.clone(),
                    parameters: params,
                    body: body.to_vec(),
                    scope: ctx.scope.clone().variables,
                });
                
                ctx.scope.define(name, func);
                Ok(None)
            },
            Stmt::If(condition, consequence, alternative) => {
                let cond_val = self.eval_expr(condition, ctx)?;
                if self.is_truthy(&cond_val) {
                    let result = self.eval_stmt_block(&consequence, ctx)?;
                    if ctx.return_flag {
                        return Ok(result);
                    }
                } else if let Some(alt) = alternative {
                    let result = self.eval_stmt_block(&alt, ctx)?;
                    if ctx.return_flag {
                        return Ok(result);
                    }
                }
                Ok(None)
            },
            Stmt::For(init, condition, increment, body) => {
                if let Some(init_stmt) = init {
                    self.eval_stmt(*init_stmt, ctx)?;
                }
                
                loop {
                    if let Some(ref cond) = condition {
                        let cond_val = self.eval_expr(cond.clone(), ctx)?;
                        if !self.is_truthy(&cond_val) {
                            break;
                        }
                    }
                    
                    let result = self.eval_stmt_block(&body, ctx)?;
                    if ctx.return_flag {
                        return Ok(result);
                    }
                    
                    if let Some(ref incr) = increment {
                        self.eval_expr(incr.clone(), ctx)?;
                    }
                }
                
                Ok(None)
            },
            Stmt::While(condition, body) => {
                loop {
                    let cond_val = self.eval_expr(condition.clone(), ctx)?;
                    if !self.is_truthy(&cond_val) {
                        break;
                    }
                    
                    let result = self.eval_stmt_block(&body, ctx)?;
                    if ctx.return_flag {
                        return Ok(result);
                    }
                }
                
                Ok(None)
            },
            Stmt::Return(expr) => {
                ctx.return_flag = true;
                if let Some(e) = expr {
                    Ok(Some(self.eval_expr(e, ctx)?))
                } else {
                    Ok(Some(JsValue::Undefined))
                }
            },
            Stmt::Block(statements) => {
                self.eval_stmt_block(&statements, ctx)
            },
            Stmt::ForIn(var_name, obj_expr, body) => {
                let obj_val = self.eval_expr(obj_expr, ctx)?;
                
                match obj_val {
                    JsValue::Object(obj) => {
                        for (key, _) in &obj.properties {
                            ctx.scope.set(var_name.clone(), JsValue::String(key.clone()));
                            self.eval_stmt_block(&body, ctx)?;
                            if ctx.return_flag {
                                return Ok(None);
                            }
                        }
                    }
                    JsValue::Array(arr) => {
                        for (i, _) in arr.iter().enumerate() {
                            ctx.scope.set(var_name.clone(), JsValue::String(i.to_string()));
                            self.eval_stmt_block(&body, ctx)?;
                            if ctx.return_flag {
                                return Ok(None);
                            }
                        }
                    }
                    _ => {}
                }
                Ok(None)
            },
        }
    }
    
    fn eval_expr(&mut self, expr: Expr, ctx: &mut ExecutionContext) -> Result<JsValue, JsError> {
        match expr {
            Expr::Identifier(name) => {
                if name == "this" {
                    return Ok(ctx.this_binding.clone());
                }
                
                if let Some(value) = ctx.scope.get(&name) {
                    Ok(value.clone())
                } else if let Some(value) = self.global_scope.get(&name).cloned() {
                    Ok(value)
                } else {
                    Err(JsError::ReferenceError(format!("Variable '{}' is not defined", name)))
                }
            },
            Expr::Number(n) => Ok(JsValue::Number(n)),
            Expr::String(s) => Ok(JsValue::String(s)),
            Expr::Boolean(b) => Ok(JsValue::Boolean(b)),
            Expr::Null => Ok(JsValue::Null),
            Expr::Undefined => Ok(JsValue::Undefined),
            Expr::BinaryOp(left, op, right) => {
                let left_val = self.eval_expr(*left, ctx)?;
                let right_val = self.eval_expr(*right, ctx)?;
                
                match op {
                    Token::Plus => {
                        let left_str = self.coerce_to_string(&left_val);
                        let right_str = self.coerce_to_string(&right_val);
                        Ok(JsValue::String(format!("{}{}", left_str, right_str)))
                    },
                    Token::Minus => {
                        let left_num = self.coerce_to_number(&left_val);
                        let right_num = self.coerce_to_number(&right_val);
                        Ok(JsValue::Number(left_num - right_num))
                    },
                    Token::Asterisk => {
                        let left_num = self.coerce_to_number(&left_val);
                        let right_num = self.coerce_to_number(&right_val);
                        Ok(JsValue::Number(left_num * right_num))
                    },
                    Token::Slash => {
                        let left_num = self.coerce_to_number(&left_val);
                        let right_num = self.coerce_to_number(&right_val);
                        Ok(JsValue::Number(left_num / right_num))
                    },
                    Token::Percent => {
                        let left_num = self.coerce_to_number(&left_val);
                        let right_num = self.coerce_to_number(&right_val);
                        Ok(JsValue::Number(left_num % right_num))
                    },
                    Token::Equal => {
                        Ok(JsValue::Boolean(left_val == right_val))
                    },
                    Token::NotEqual => {
                        Ok(JsValue::Boolean(left_val != right_val))
                    },
                    Token::LessThan => {
                        let left_num = self.coerce_to_number(&left_val);
                        let right_num = self.coerce_to_number(&right_val);
                        Ok(JsValue::Boolean(left_num < right_num))
                    },
                    Token::LessThanOrEqual => {
                        let left_num = self.coerce_to_number(&left_val);
                        let right_num = self.coerce_to_number(&right_val);
                        Ok(JsValue::Boolean(left_num <= right_num))
                    },
                    Token::GreaterThan => {
                        let left_num = self.coerce_to_number(&left_val);
                        let right_num = self.coerce_to_number(&right_val);
                        Ok(JsValue::Boolean(left_num > right_num))
                    },
                    Token::GreaterThanOrEqual => {
                        let left_num = self.coerce_to_number(&left_val);
                        let right_num = self.coerce_to_number(&right_val);
                        Ok(JsValue::Boolean(left_num >= right_num))
                    },
                    Token::LogicalAnd => {
                        let left_bool = self.is_truthy(&left_val);
                        if !left_bool {
                            Ok(JsValue::Boolean(false))
                        } else {
                            Ok(JsValue::Boolean(self.is_truthy(&right_val)))
                        }
                    },
                    Token::LogicalOr => {
                        let left_bool = self.is_truthy(&left_val);
                        if left_bool {
                            Ok(JsValue::Boolean(true))
                        } else {
                            Ok(JsValue::Boolean(self.is_truthy(&right_val)))
                        }
                    },
                    _ => Err(JsError::TypeError("Unsupported binary operation".to_string())),
                }
            },
            Expr::Call(callee, args) => {
                let func_val = self.eval_expr(*callee, ctx)?;
                
                match func_val {
                    JsValue::Function(func) => {
                        let parent_scope = Box::new(ctx.scope.clone());
                        let mut func_scope = Scope::new(Some(parent_scope));
                        
                        for (param, arg) in func.parameters.iter().zip(args.iter()) {
                            let arg_val = self.eval_expr(arg.clone(), ctx)?;
                            func_scope.define(param.clone(), arg_val);
                        }
                        
                        let mut func_ctx = ExecutionContext::new(func_scope, JsValue::Undefined);
                        
                        for stmt in &func.body {
                            match self.eval_stmt(stmt.clone(), &mut func_ctx)? {
                                Some(val) if func_ctx.return_flag => {
                                    return Ok(val);
                                }
                                _ => {}
                            }
                        }
                        
                        Ok(JsValue::Undefined)
                    },
                    JsValue::Object(obj) => {
                        let class_name = obj.class_name.clone();
                        let func = self.built_in_functions.get(&class_name).cloned();
                        
                        if let Some(built_in_func) = func {
                            let mut arg_vals = Vec::new();
                            for arg in args {
                                arg_vals.push(self.eval_expr(arg, ctx)?);
                            }
                            
                            match built_in_func {
                                BuiltInFunction::Native(f) => f(&arg_vals),
                            }
                        } else {
                            Err(JsError::TypeError(format!("Object of type '{}' is not callable", class_name)))
                        }
                    },
                    _ => Err(JsError::TypeError("Attempted to call a non-function".to_string())),
                }
            },
            Expr::Member(obj_expr, prop_name) => {
                let obj_val = self.eval_expr(*obj_expr, ctx)?;
                
                match obj_val {
                    JsValue::Object(obj) => {
                        if let Some(prop_val) = obj.get_property(&prop_name) {
                            Ok(prop_val.clone())
                        } else {
                            Ok(JsValue::Undefined)
                        }
                    },
                    JsValue::String(s) => {
                        if let Some(proto) = self.prototypes.get("String") {
                            if let Some(method) = proto.methods.get(&prop_name) {
                                Ok(JsValue::Function(method.clone()))
                            } else {
                                Ok(JsValue::Undefined)
                            }
                        } else {
                            Ok(JsValue::Undefined)
                        }
                    },
                    _ => Ok(JsValue::Undefined),
                }
            },
            Expr::Array(elements) => {
                let mut arr_vals = Vec::new();
                for elem in elements {
                    arr_vals.push(self.eval_expr(elem, ctx)?);
                }
                Ok(JsValue::Array(arr_vals))
            },
            Expr::Object(props) => {
                let mut obj_props = HashMap::new();
                for (key, value) in props {
                    let val = self.eval_expr(value, ctx)?;
                    obj_props.insert(key, val);
                }
                
                let mut obj = JsObject::new("Object".to_string());
                obj.properties = obj_props;
                Ok(JsValue::Object(obj))
            },
            Expr::Index(obj_expr, index_expr) => {
                let obj = self.eval_expr(*obj_expr, ctx)?;
                let index = self.eval_expr(*index_expr, ctx)?;
                
                match obj {
                    JsValue::Array(arr) => {
                        if let JsValue::Number(n) = index {
                            let idx = n as usize;
                            Ok(arr.get(idx).cloned().unwrap_or(JsValue::Undefined))
                        } else {
                            Ok(JsValue::Undefined)
                        }
                    },
                    JsValue::String(s) => {
                        if let JsValue::Number(n) = index {
                            let idx = n as usize;
                            Ok(s.chars().nth(idx)
                                .map(|c| JsValue::String(c.to_string()))
                                .unwrap_or(JsValue::Undefined))
                        } else {
                            Ok(JsValue::Undefined)
                        }
                    },
                    _ => Ok(JsValue::Undefined)
                }
            },
            Expr::New(constructor, args) => {
                let ctor = self.eval_expr(*constructor, ctx)?;
                
                let mut new_obj = JsObject::new("Object".to_string());
                
                if let JsValue::Function(func) = ctor {
                    let parent_scope = Box::new(ctx.scope.clone());
                    let mut func_scope = Scope::new(Some(parent_scope));
                    
                    func_scope.define("this".to_string(), JsValue::Object(new_obj.clone()));
                    
                    for (param, arg) in func.parameters.iter().zip(args.iter()) {
                        let arg_val = self.eval_expr(arg.clone(), ctx)?;
                        func_scope.define(param.clone(), arg_val);
                    }
                    
                    let mut func_ctx = ExecutionContext::new(func_scope, JsValue::Object(new_obj.clone()));
                    for stmt in &func.body {
                        self.eval_stmt(stmt.clone(), &mut func_ctx)?;
                    }
                    
                    Ok(JsValue::Object(new_obj))
                } else {
                    Err(JsError::TypeError("Not a constructor".to_string()))
                }
            },
            Expr::Typeof(expr) => {
                let val = self.eval_expr(*expr, ctx)?;
                let type_str = typeof_value(&val);
                Ok(JsValue::String(type_str))
            },
            Expr::FunctionDef(_, _, _) => {
                Ok(JsValue::Undefined)
            },
        }
    }
    
    fn is_truthy(&self, value: &JsValue) -> bool {
        match value {
            JsValue::Null | JsValue::Undefined => false,
            JsValue::Boolean(b) => *b,
            JsValue::Number(n) => !n.is_nan() && *n != 0.0,
            JsValue::String(s) => !s.is_empty(),
            JsValue::Array(_) | JsValue::Object(_) => true,
            _ => true,
        }
    }
    
    fn coerce_to_number(&self, value: &JsValue) -> f64 {
        match value {
            JsValue::Number(n) => *n,
            JsValue::Boolean(b) => if *b { 1.0 } else { 0.0 },
            JsValue::String(s) => s.parse().unwrap_or(std::f64::NAN),
            JsValue::Null => 0.0,
            JsValue::Undefined => std::f64::NAN,
            _ => std::f64::NAN,
        }
    }
    
    fn coerce_to_string(&self, value: &JsValue) -> String {
        match value {
            JsValue::String(s) => s.clone(),
            JsValue::Number(n) => n.to_string(),
            JsValue::Boolean(b) => b.to_string(),
            JsValue::Null => "null".to_string(),
            JsValue::Undefined => "undefined".to_string(),
            JsValue::Object(_) => "[object Object]".to_string(),
            JsValue::Array(arr) => arr.iter()
                .map(|v| self.coerce_to_string(v))
                .collect::<Vec<_>>()
                .join(","),
            _ => String::new(),
        }
    }
    
    pub fn eval_expression(&mut self, expr: &[u8]) -> Result<JsValue, JsError> {
        self.execute(expr)
    }
    
    pub fn call_function(&mut self, name: &str, args: &[JsValue]) -> Result<JsValue, JsError> {
        if let Some(func) = self.built_in_functions.get(name) {
            match func {
                BuiltInFunction::Native(f) => f(args),
            }
        } else {
            Err(JsError::ReferenceError(format!("Function {} is not defined", name)))
        }
    }
    
    pub fn register_event_listener(&mut self, event_type: String, listener: JsFunction) {
        self.event_listeners.insert(event_type, listener);
    }
    
    pub fn dispatch_event(&mut self, event_type: &str, _target: &str) -> Result<JsValue, JsError> {
        if let Some(listener) = self.event_listeners.get(event_type) {
            Ok(JsValue::Undefined)
        } else {
            Ok(JsValue::Undefined)
        }
    }
    
    pub fn handle_error(&mut self, error: JsError) {
        let error_msg = format!("{:?}", error);
        self.error_buffer.push(error_msg);
    }
    
    /// Clear the output buffer
    pub fn clear_output(&mut self) {
        self.output_buffer.clear();
    }
    
    /// Get the current output buffer content
    pub fn get_output(&self) -> String {
        self.output_buffer.clone()
    }
    
    /// Append content to the output buffer (used by document.write)
    pub fn append_output(&mut self, content: &str) {
        self.output_buffer.push_str(content);
    }
    
    pub fn update_live_collections(&mut self) {
        // Live collections are not implemented in this version
        // The document collections are managed by TrussCore directly
    }
    
    pub fn document_write(&mut self, html: &str) -> Result<(), JsError> {
        if let Some(ref bridge) = self.trusscore_bridge {
            let html_clone = html.to_string();
            let bridge_clone = bridge.clone();
            std::thread::spawn(move || {
                let mut trusscore = bridge_clone.lock().unwrap();
                if let Err(e) = trusscore.document_write(&html_clone) {
                    eprintln!("Error writing document: {}", e);
                }
            });
            Ok(())
        } else {
            Err(JsError::InternalError("TrussCore bridge not initialized".to_string()))
        }
    }
}

#[derive(Debug, Clone)]
pub enum BuiltInFunction {
    Native(fn(&[JsValue]) -> Result<JsValue, JsError>),
}

#[derive(Debug, Clone)]
pub struct Document {
    pub title: String,
    pub url: String,
    pub referrer: String,
    pub last_modified: String,
    pub domain: String,
    pub cookie: String,
    pub bg_color: String,
    pub fg_color: String,
    pub link_color: String,
    pub vlink_color: String,
    pub alink_color: String,
    pub forms: Vec<String>,
    pub images: Vec<String>,
    pub links: Vec<String>,
    pub anchors: Vec<String>,
    pub applets: Vec<String>,
    pub embeds: Vec<String>,
}

impl Document {
    pub fn new() -> Self {
        Document {
            title: String::new(),
            url: String::new(),
            referrer: String::new(),
            last_modified: String::new(),
            domain: String::new(),
            cookie: String::new(),
            bg_color: String::new(),
            fg_color: String::new(),
            link_color: String::new(),
            vlink_color: String::new(),
            alink_color: String::new(),
            forms: Vec::new(),
            images: Vec::new(),
            links: Vec::new(),
            anchors: Vec::new(),
            applets: Vec::new(),
            embeds: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Location {
    pub href: String,
    pub protocol: String,
    pub host: String,
    pub hostname: String,
    pub port: String,
    pub pathname: String,
    pub search: String,
    pub hash: String,
}

impl Location {
    pub fn new() -> Self {
        Location {
            href: String::new(),
            protocol: String::new(),
            host: String::new(),
            hostname: String::new(),
            port: String::new(),
            pathname: String::new(),
            search: String::new(),
            hash: String::new(),
        }
    }
    
    pub fn assign(&mut self, url: &str) {
        self.href = url.to_string();
    }
    
    pub fn reload(&self) {
    }
}

#[derive(Debug, Clone)]
pub struct History {
    pub length: usize,
    pub states: Vec<String>,
    pub current_index: usize,
}

impl History {
    pub fn new() -> Self {
        History {
            length: 0,
            states: Vec::new(),
            current_index: 0,
        }
    }
    
    pub fn back(&mut self) {
        if self.current_index > 0 {
            self.current_index -= 1;
        }
    }
    
    pub fn forward(&mut self) {
        if self.current_index < self.states.len() - 1 {
            self.current_index += 1;
        }
    }
    
    pub fn go(&mut self, delta: i32) {
        let new_index = self.current_index as i32 + delta;
        if new_index >= 0 && (new_index as usize) < self.states.len() {
            self.current_index = new_index as usize;
        }
    }
}

#[derive(Debug, Clone)]
pub struct Navigator {
    pub app_name: String,
    pub app_version: String,
    pub app_code_name: String,
    pub platform: String,
    pub user_agent: String,
    pub vendor: String,
    pub vendor_sub: String,
    pub product: String,
    pub product_sub: String,
    pub languages: Vec<String>,
    pub on_line: bool,
    pub cookie_enabled: bool,
    pub java_enabled: bool,
    pub mime_types: Vec<String>,
    pub plugins: Vec<String>,
}

impl Navigator {
    pub fn new() -> Self {
        Navigator {
            app_name: "Retro1996".to_string(),
            app_version: "3.0 (TrussCore/1.0; Win95)".to_string(),
            app_code_name: "Mozilla".to_string(),
            platform: "Win95".to_string(),
            user_agent: "Mozilla/3.0 (compatible; Retro1996/3.0; TrussCore/1.0; Win95)".to_string(),
            vendor: String::new(),
            vendor_sub: String::new(),
            product: "TrussCore".to_string(),
            product_sub: "19960101".to_string(),
            languages: vec!["en-US".to_string(), "en".to_string()],
            on_line: true,
            cookie_enabled: true,
            java_enabled: false,
            mime_types: Vec::new(),
            plugins: Vec::new(),
        }
    }
    
    pub fn java_enabled(&self) -> bool {
        self.java_enabled
    }
    
    pub fn mime_types(&self) -> &[String] {
        &self.mime_types
    }
    
    pub fn plugins(&self) -> &[String] {
        &self.plugins
    }
}

#[derive(Debug, Clone)]
pub struct Screen {
    pub width: u32,
    pub height: u32,
    pub avail_width: u32,
    pub avail_height: u32,
    pub color_depth: u8,
    pub pixel_depth: u8,
}

impl Screen {
    pub fn new() -> Self {
        Screen {
            width: 800,
            height: 600,
            avail_width: 800,
            avail_height: 600,
            color_depth: 16,
            pixel_depth: 16,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Window {
    pub document: Document,
    pub location: Location,
    pub history: History,
    pub navigator: Navigator,
    pub screen: Screen,
    pub frames: Vec<Window>,
    pub parent: Option<Box<Window>>,
    pub opener: Option<Box<Window>>,
    pub top: Option<Box<Window>>,
    pub name: String,
    pub status: String,
    pub default_status: String,
    pub closed: bool,
}

impl Window {
    pub fn new() -> Self {
        Window {
            document: Document::new(),
            location: Location::new(),
            history: History::new(),
            navigator: Navigator::new(),
            screen: Screen::new(),
            frames: Vec::new(),
            parent: None,
            opener: None,
            top: None,
            name: String::new(),
            status: String::new(),
            default_status: String::new(),
            closed: false,
        }
    }
    
    pub fn alert(&self, message: &str) {
        println!("Alert: {}", message);
    }
    
    pub fn confirm(&self, message: &str) -> bool {
        println!("Confirm: {}", message);
        true
    }
    
    pub fn prompt(&self, message: &str, default: &str) -> Option<String> {
        println!("Prompt: {}", message);
        Some(default.to_string())
    }
    
    pub fn close(&mut self) {
        self.closed = true;
    }
    
    pub fn focus(&self) {
    }
    
    pub fn blur(&self) {
    }
    
    pub fn scroll_to(&self, _x: u32, _y: u32) {
    }
    
    pub fn scroll_by(&self, _x: i32, _y: i32) {
    }
}

fn alert(args: &[JsValue]) -> Result<JsValue, JsError> {
    if let Some(JsValue::String(msg)) = args.get(0) {
        println!("Alert: {}", msg);
    }
    Ok(JsValue::Undefined)
}

fn confirm(args: &[JsValue]) -> Result<JsValue, JsError> {
    if let Some(JsValue::String(msg)) = args.get(0) {
        println!("Confirm: {}", msg);
    }
    Ok(JsValue::Boolean(true))
}

fn prompt(args: &[JsValue]) -> Result<JsValue, JsError> {
    if let (Some(JsValue::String(msg)), Some(JsValue::String(default))) = (args.get(0), args.get(1)) {
        println!("Prompt: {}", msg);
        return Ok(JsValue::String(default.clone()));
    }
    Ok(JsValue::Null)
}

fn parse_int(args: &[JsValue]) -> Result<JsValue, JsError> {
    if let Some(JsValue::String(s)) = args.get(0) {
        if let Ok(num) = s.parse::<f64>() {
            return Ok(JsValue::Number(num));
        }
    }
    Ok(JsValue::Number(std::f64::NAN))
}

fn parse_float(args: &[JsValue]) -> Result<JsValue, JsError> {
    if let Some(JsValue::String(s)) = args.get(0) {
        if let Ok(num) = s.parse::<f64>() {
            return Ok(JsValue::Number(num));
        }
    }
    Ok(JsValue::Number(std::f64::NAN))
}

fn is_nan(args: &[JsValue]) -> Result<JsValue, JsError> {
    if let Some(JsValue::Number(n)) = args.get(0) {
        return Ok(JsValue::Boolean(n.is_nan()));
    }
    Ok(JsValue::Boolean(true))
}

fn is_finite(args: &[JsValue]) -> Result<JsValue, JsError> {
    if let Some(JsValue::Number(n)) = args.get(0) {
        return Ok(JsValue::Boolean(n.is_finite()));
    }
    Ok(JsValue::Boolean(false))
}

fn eval(args: &[JsValue]) -> Result<JsValue, JsError> {
    if let Some(JsValue::String(code)) = args.get(0) {
        return Ok(JsValue::Undefined);
    }
    Ok(JsValue::Undefined)
}

fn escape(args: &[JsValue]) -> Result<JsValue, JsError> {
    if let Some(JsValue::String(s)) = args.get(0) {
        let (bytes, _, _) = encoding_rs::WINDOWS_1252.encode(s);
        
        let escaped = bytes.iter().map(|&byte| {
            let c = byte as char;
            if c.is_ascii_alphanumeric() || "@*_+-./".contains(c) {
                c.to_string()
            } else {
                format!("%{:02X}", byte)
            }
        }).collect::<String>();
        return Ok(JsValue::String(escaped));
    }
    Ok(JsValue::String(String::new()))
}

fn unescape(args: &[JsValue]) -> Result<JsValue, JsError> {
    if let Some(JsValue::String(s)) = args.get(0) {
        let mut result = Vec::new();
        let mut chars = s.chars().peekable();
        
        while let Some(c) = chars.next() {
            if c == '%' {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte);
                } else {
                    result.push(c as u8);
                }
            } else {
                result.push(c as u8);
            }
        }
        
        let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(&result);
        return Ok(JsValue::String(decoded.into_owned()));
    }
    Ok(JsValue::String(String::new()))
}

fn string_from_char_code(args: &[JsValue]) -> Result<JsValue, JsError> {
    let codes: Vec<f64> = args.iter()
        .filter_map(|v| {
            if let JsValue::Number(n) = v {
                Some(*n)
            } else {
                None
            }
        })
        .collect();
    
    let bytes: Vec<u8> = codes.iter()
        .map(|&code| (code as u32 % 256) as u8)
        .collect();
    
    let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
    Ok(JsValue::String(decoded.into_owned()))
}

pub fn typeof_value(value: &JsValue) -> String {
    match value {
        JsValue::Undefined => "undefined".to_string(),
        JsValue::Null => "object".to_string(),
        JsValue::Boolean(_) => "boolean".to_string(),
        JsValue::Number(n) => if n.is_nan() { "number".to_string() } else { "number".to_string() },
        JsValue::String(_) => "string".to_string(),
        JsValue::Object(_) => "object".to_string(),
        JsValue::Array(_) => "object".to_string(),
        JsValue::Function(_) => "function".to_string(),
    }
}