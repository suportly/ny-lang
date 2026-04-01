pub mod token;

use crate::common::{CompileError, Span};
use token::{Token, TokenKind};

pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    start: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            pos: 0,
            start: 0,
        }
    }

    fn byte_pos(&self, char_pos: usize) -> usize {
        let clamped = char_pos.min(self.source.len());
        self.source[..clamped].iter().map(|c| c.len_utf8()).sum()
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.source.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.source.get(self.pos).copied();
        self.pos += 1;
        c
    }

    fn make_span(&self) -> Span {
        Span::new(self.byte_pos(self.start), self.byte_pos(self.pos))
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(' ' | '\t' | '\r' | '\n') => {
                    self.advance();
                }
                Some('/') if self.peek_next() == Some('/') => {
                    while self.peek().is_some_and(|c| c != '\n') {
                        self.advance();
                    }
                }
                Some('/') if self.peek_next() == Some('*') => {
                    self.advance(); // consume /
                    self.advance(); // consume *
                    let mut depth = 1u32;
                    while depth > 0 {
                        match (self.peek(), self.peek_next()) {
                            (Some('/'), Some('*')) => {
                                self.advance();
                                self.advance();
                                depth += 1;
                            }
                            (Some('*'), Some('/')) => {
                                self.advance();
                                self.advance();
                                depth -= 1;
                            }
                            (Some(_), _) => {
                                self.advance();
                            }
                            (None, _) => break, // unterminated — will be caught by next_token
                        }
                    }
                }
                _ => break,
            }
        }
    }

    fn read_number(&mut self) -> TokenKind {
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.advance();
        }
        if self.peek() == Some('.') && self.peek_next().is_some_and(|c| c.is_ascii_digit()) {
            self.advance();
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.advance();
            }
            let text: String = self.source[self.start..self.pos].iter().collect();
            TokenKind::FloatLit(text.parse().unwrap())
        } else {
            let text: String = self.source[self.start..self.pos].iter().collect();
            TokenKind::IntLit(text.parse().unwrap())
        }
    }

    fn read_string(&mut self) -> Result<TokenKind, CompileError> {
        let mut value = String::new();
        loop {
            match self.advance() {
                None => {
                    return Err(CompileError::syntax(
                        "unterminated string literal".to_string(),
                        self.make_span(),
                    ));
                }
                Some('"') => break,
                Some('\\') => match self.advance() {
                    Some('n') => value.push('\n'),
                    Some('t') => value.push('\t'),
                    Some('\\') => value.push('\\'),
                    Some('"') => value.push('"'),
                    Some('0') => value.push('\0'),
                    Some(c) => {
                        return Err(CompileError::syntax(
                            format!("unknown escape sequence '\\{}'", c),
                            self.make_span(),
                        ));
                    }
                    None => {
                        return Err(CompileError::syntax(
                            "unterminated escape sequence".to_string(),
                            self.make_span(),
                        ));
                    }
                },
                Some(c) => value.push(c),
            }
        }
        Ok(TokenKind::StringLit(value))
    }

    fn read_ident_or_keyword(&mut self) -> TokenKind {
        while self.peek().is_some_and(|c| c.is_alphanumeric() || c == '_') {
            self.advance();
        }
        let text: String = self.source[self.start..self.pos].iter().collect();
        match text.as_str() {
            "fn" => TokenKind::Fn,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "return" => TokenKind::Return,
            "struct" => TokenKind::Struct,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "as" => TokenKind::As,
            "true" => TokenKind::BoolLit(true),
            "false" => TokenKind::BoolLit(false),
            _ => TokenKind::Ident(text),
        }
    }

    fn next_token(&mut self) -> Result<Token, CompileError> {
        self.skip_whitespace_and_comments();
        self.start = self.pos;

        let c = match self.advance() {
            Some(c) => c,
            None => return Ok(Token::new(TokenKind::Eof, self.make_span())),
        };

        let kind = match c {
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semi,
            '~' => TokenKind::Tilde,
            '+' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::PlusAssign
                } else {
                    TokenKind::Plus
                }
            }
            '/' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::SlashAssign
                } else {
                    TokenKind::Slash
                }
            }
            '%' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::PercentAssign
                } else {
                    TokenKind::Percent
                }
            }
            '*' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::StarAssign
                } else {
                    TokenKind::Star
                }
            }
            '^' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::CaretAssign
                } else {
                    TokenKind::Caret
                }
            }
            '&' => {
                if self.peek() == Some('&') {
                    self.advance();
                    TokenKind::And
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::AmpAssign
                } else {
                    TokenKind::Ampersand
                }
            }
            '.' => {
                if self.peek() == Some('.') {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::DotDotEq
                    } else {
                        TokenKind::DotDot
                    }
                } else {
                    TokenKind::Dot
                }
            }
            ':' => {
                if self.peek() == Some('~') {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::ColonTildeAssign
                    } else {
                        TokenKind::ColonTilde
                    }
                } else if self.peek() == Some(':') {
                    self.advance();
                    TokenKind::ColonColon
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::ColonAssign
                } else {
                    TokenKind::Colon
                }
            }
            '-' => {
                if self.peek() == Some('>') {
                    self.advance();
                    TokenKind::Arrow
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::MinusAssign
                } else {
                    TokenKind::Minus
                }
            }
            '=' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::Eq
                } else {
                    TokenKind::Assign
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::Ne
                } else {
                    TokenKind::Not
                }
            }
            '<' => {
                if self.peek() == Some('<') {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::LtLtAssign
                    } else {
                        TokenKind::LtLt
                    }
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::Le
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                if self.peek() == Some('>') {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::GtGtAssign
                    } else {
                        TokenKind::GtGt
                    }
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::Ge
                } else {
                    TokenKind::Gt
                }
            }
            '|' => {
                if self.peek() == Some('|') {
                    self.advance();
                    TokenKind::Or
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::PipeAssign
                } else {
                    TokenKind::Pipe
                }
            }
            '"' => {
                return self
                    .read_string()
                    .map(|kind| Token::new(kind, self.make_span()));
            }
            c if c.is_ascii_digit() => {
                self.pos -= 1;
                self.read_number()
            }
            c if c.is_alphabetic() || c == '_' => {
                self.pos -= 1;
                self.read_ident_or_keyword()
            }
            c => {
                return Err(CompileError::syntax(
                    format!("unexpected character '{}'", c),
                    self.make_span(),
                ));
            }
        };

        Ok(Token::new(kind, self.make_span()))
    }
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, Vec<CompileError>> {
    let mut lexer = Lexer::new(source);
    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    loop {
        match lexer.next_token() {
            Ok(token) => {
                let is_eof = token.kind == TokenKind::Eof;
                tokens.push(token);
                if is_eof {
                    break;
                }
            }
            Err(e) => {
                errors.push(e);
            }
        }
    }

    if errors.is_empty() {
        Ok(tokens)
    } else {
        Err(errors)
    }
}
