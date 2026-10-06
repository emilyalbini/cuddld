use crate::picohcl::ast::{Span, Spanned};
use cuddld_macros::{Display, Error};
use std::collections::VecDeque;
use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Token {
    Literal(String),
    RawString(String),
    OpenString,
    CloseString,
    OpenParen,
    CloseParen,
    OpenSquare,
    CloseSquare,
    OpenCurly,
    CloseCurly,
    Equal,
    Dot,
    Comma,
    Newline,
    EndOfInput,
}

pub(super) struct Lexer<'a> {
    input: Peekable<Chars<'a>>,
    span: Span,
    next_span: Span,
    string_state: Vec<StringState>,
    curly_level: u32,
    queued_tokens: VecDeque<Result<Spanned<Token>, LexerError>>,
    peeked_tokens: VecDeque<Result<Spanned<Token>, LexerError>>,
}

impl<'a> Lexer<'a> {
    pub(super) fn new(input: &'a str) -> Self {
        Self {
            input: input.chars().peekable(),
            span: Span { line: 0, col: 0 },
            next_span: Span { line: 0, col: 0 },
            string_state: Vec::new(),
            curly_level: 0,
            queued_tokens: VecDeque::new(),
            peeked_tokens: VecDeque::new(),
        }
    }
}

impl Lexer<'_> {
    pub(super) fn next(&mut self) -> Result<Spanned<Token>, LexerError> {
        if let Some(peeked) = self.peeked_tokens.pop_front() {
            return peeked;
        }
        if let Some(queued) = self.queued_tokens.pop_front() {
            return queued;
        }

        'lexer: loop {
            if let Some(StringState::Raw) = self.string_state.last() {
                let mut string = String::new();
                let mut escape = false;
                loop {
                    let Some(chr) = self.next_char() else {
                        return Err(self.err(LexerErrorKind::UnterminatedString));
                    };
                    if escape {
                        escape = false;
                        match chr {
                            'n' => string.push('\n'),
                            't' => string.push('\t'),
                            '"' => string.push('"'),
                            '$' => string.push('$'),
                            '\\' => string.push('\\'),
                            _ => return Err(self.err(LexerErrorKind::InvalidEscape(chr))),
                        }
                    } else if chr == '\\' {
                        escape = true;
                    } else if chr == '$' && self.peek_char() == Some('{') {
                        self.next_char();
                        *self.string_state.last_mut().unwrap() =
                            StringState::Interpolation(self.curly_level);
                        return Ok(self.spanned(Token::RawString(string)));
                    } else if chr == '"' {
                        self.string_state.pop();
                        self.queued_tokens.push_back(Ok(self.spanned(Token::CloseString)));
                        return Ok(self.spanned(Token::RawString(string)));
                    } else {
                        string.push(chr);
                    }
                }
            }

            match self.next_char() {
                None => return Ok(self.spanned(Token::EndOfInput)),

                // Newline
                Some('\n') => return Ok(self.spanned(Token::Newline)),

                // Symbols
                Some('[') => return Ok(self.spanned(Token::OpenSquare)),
                Some(']') => return Ok(self.spanned(Token::CloseSquare)),
                Some('(') => return Ok(self.spanned(Token::OpenParen)),
                Some(')') => return Ok(self.spanned(Token::CloseParen)),
                Some('=') => return Ok(self.spanned(Token::Equal)),
                Some('.') => return Ok(self.spanned(Token::Dot)),
                Some(',') => return Ok(self.spanned(Token::Comma)),

                Some('{') => {
                    self.curly_level += 1;
                    return Ok(self.spanned(Token::OpenCurly));
                }
                Some('}') => {
                    if let Some(state) = self.string_state.last_mut()
                        && let StringState::Interpolation(curly_level) = state
                    {
                        // Prevent the curly close at the wrong nesting depth to close.
                        if *curly_level == self.curly_level {
                            *state = StringState::Raw;
                            continue 'lexer;
                        }
                    }
                    self.curly_level = self.curly_level.saturating_sub(1);
                    return Ok(self.spanned(Token::CloseCurly));
                }

                // Literals
                Some(c) if is_literal_start(c) => {
                    let mut literal = String::from(c);
                    while let Some(n) = self.peek_char()
                        && is_literal_continue(n)
                    {
                        self.next_char();
                        literal.push(n);
                    }
                    return Ok(self.spanned(Token::Literal(literal)));
                }

                // Strings
                Some('"') => {
                    self.string_state.push(StringState::Raw);
                    return Ok(self.spanned(Token::OpenString));
                }

                // Whitespace
                Some(' ' | '\r' | '\t') => {}

                // Block comment
                Some('/') if self.peek_char() == Some('*') => {
                    self.next_char();
                    loop {
                        if self.peek_char().is_none() {
                            return Err(self.err(LexerErrorKind::UnterminatedComment));
                        }
                        if self.next_char() == Some('*') && self.next_char() == Some('/') {
                            continue 'lexer;
                        }
                    }
                }

                // Line comment
                Some('/') if self.peek_char() == Some('/') => {
                    self.next_char(); // Consume the /.
                    loop {
                        let next = self.next_char();
                        if next == Some('\n') {
                            return Ok(self.spanned(Token::Newline));
                        } else if next.is_none() {
                            return Ok(self.spanned(Token::EndOfInput));
                        }
                    }
                }

                Some(chr) => return Err(self.err(LexerErrorKind::InvalidChar(chr))),
            }
        }
    }

    pub(super) fn peek(&mut self) -> Result<Spanned<Token>, LexerError> {
        if self.peeked_tokens.is_empty() {
            let token = self.next();
            self.peeked_tokens.push_back(token);
        }
        self.peeked_tokens.front().unwrap().clone()
    }

    fn next_char(&mut self) -> Option<char> {
        self.span = self.next_span;
        match self.input.next() {
            Some('\n') => {
                self.next_span.line += 1;
                self.next_span.col = 0;
                Some('\n')
            }
            Some(c) => {
                self.next_span.col += 1;
                Some(c)
            }
            None => None,
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        self.input.peek().copied()
    }

    fn err(&self, kind: LexerErrorKind) -> LexerError {
        LexerError { span: self.span, kind }
    }

    fn spanned<T>(&self, item: T) -> Spanned<T> {
        Spanned { item, span: self.span }
    }
}

enum StringState {
    Raw,
    Interpolation(u32),
}

#[derive(Debug, Error, Display, Clone)]
#[display("{span:?}: {kind}")]
pub(super) struct LexerError {
    pub(super) span: Span,
    pub(super) kind: LexerErrorKind,
}

#[derive(Debug, Display, Clone)]
pub(super) enum LexerErrorKind {
    #[display("unterminated string")]
    UnterminatedString,
    #[display("unterminated comment")]
    UnterminatedComment,
    #[display("invalid char: {f0:?}")]
    InvalidChar(char),
    #[display("invalid escape: \\{f0}")]
    InvalidEscape(char),
}

fn is_literal_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_literal_continue(c: char) -> bool {
    is_literal_start(c) || c.is_ascii_digit() || c == '-'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn lex(input: &str) -> String {
        let mut lexer = Lexer::new(input);
        let mut result = Vec::new();
        loop {
            match lexer.next() {
                Ok(token @ Spanned { item: Token::EndOfInput, .. }) => {
                    result.push(token);
                    break;
                }
                Ok(token) => result.push(token),
                Err(err) => panic!("lexing failed: {err}"),
            }
        }
        format!("{result:#?}")
    }

    #[test]
    fn test_lexer() {
        let test = "hello =  ()  /* [] */ . // hello \n[\"world\\n\\\\\", foo.bar}{]\nBAZ";
        assert_snapshot!(lex(test));
    }

    #[test]
    fn test_lex_string_interpolation() {
        let test = "hello = \"world ${foo.bar}\"";
        assert_snapshot!(lex(test));
    }

    #[test]
    fn test_lex_nested_string_interpolation() {
        let test = "hello = \"world ${foo \"bar ${{ baz }}\"}\"";
        assert_snapshot!(lex(test));
    }

    #[test]
    fn test_lex_empty_string_interpolation() {
        let test = "hello = \"${foo}\"";
        assert_snapshot!(lex(test));
    }
}
