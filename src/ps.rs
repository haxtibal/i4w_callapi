use serde::Serialize;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq)]
pub enum Error {
    LexerError,
    ParserError,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::LexerError => write!(f, "failed to lex PowerShell syntax"),
            Error::ParserError => write!(f, "failed to parse PowerShell syntax"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, PartialEq, PartialOrd)]
enum Token {
    String(String),
    Number(String),
    Bool(bool),
    Comma,
    ArrayBegin,
    ArrayEnd,
    ArrayOpBegin,
    ArrayOpEnd,
}

#[derive(Debug, PartialEq, PartialOrd, Serialize)]
#[serde(untagged)]
pub enum Number {
    PosInt(u64),
    NegInt(i64),
    Float(f64),
}

impl Number {
    fn parse(number: &str) -> Option<Number> {
        if let Some(first_char) = number.chars().next() {
            if first_char == '-' {
                if let Ok(signed) = number.parse::<i64>() {
                    return Some(Number::NegInt(signed));
                }
            } else if let Ok(unsigned) = number.parse::<u64>() {
                return Some(Number::PosInt(unsigned));
            }
        }
        if let Ok(float) = number.parse::<f64>() {
            Some(Number::Float(float))
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Serialize)]
#[serde(untagged)]
pub enum CliArgument {
    Array(Vec<CliArgument>),
    Bool(bool),
    Number(Number),
    String(String),
}

pub fn from_str(input: &str) -> Result<CliArgument> {
    let lexer = Lexer::from_str(input);
    let tokens = lexer.lex()?;
    let mut parser = Parser { input: &tokens };
    parser.parse_argument()
}

#[derive(Debug, PartialEq)]
enum LexerState {
    Control,
    SingleQuote,
    DoubleQuote,
    MaybeArrayOp,
    ParanthesesCmd,
}

struct Lexer<'a> {
    input: &'a str,
    tokens: Vec<Token>,
    state: LexerState,
    escaping: bool,
    buf: String,
}

impl<'a> Lexer<'a> {
    pub fn from_str(input: &'a str) -> Self {
        Lexer {
            input,
            tokens: Vec::new(),
            state: LexerState::Control,
            escaping: false,
            buf: String::new(),
        }
    }

    pub fn lex(mut self) -> Result<Vec<Token>> {
        while !self.input.is_empty() {
            if let Err(error) = match self.state {
                LexerState::Control => self.scan_control(),
                LexerState::SingleQuote => self.scan_singlequote(),
                LexerState::DoubleQuote => self.scan_doublequote(),
                LexerState::MaybeArrayOp => self.scan_maybearrayop(),
                LexerState::ParanthesesCmd => self.scan_parantheses_cmd(),
            } {
                return Err(error);
            }
        }
        self.store_buf_as_token();
        Ok(self.tokens)
    }

    fn scan_control(&mut self) -> Result<()> {
        if let Some(peeked_char) = self.input.chars().next() {
            self.eat(1);
            if self.escaping {
                self.buf.push(peeked_char);
                self.escaping = false;
            } else if peeked_char == '"' {
                self.state = LexerState::DoubleQuote;
            } else if peeked_char == '\'' {
                self.state = LexerState::SingleQuote;
            } else if matches!(peeked_char, ' ' | '\t' | '\r') {
            } else if peeked_char == '[' {
                self.tokens.push(Token::ArrayBegin);
            } else if peeked_char == ']' {
                self.store_buf_as_token();
                self.tokens.push(Token::ArrayEnd);
            } else if peeked_char == '(' {
                self.buf.push(peeked_char);
                self.state = LexerState::ParanthesesCmd;
            } else if peeked_char == ')' {
                self.store_buf_as_token();
                self.tokens.push(Token::ArrayOpEnd);
            } else if peeked_char == '`' {
                self.escaping = true;
            } else if peeked_char == ',' {
                self.store_buf_as_token();
                self.tokens.push(Token::Comma);
            } else if peeked_char == '@' {
                self.state = LexerState::MaybeArrayOp;
            } else {
                self.buf.push(peeked_char);
            }
        }
        Ok(())
    }

    fn scan_singlequote(&mut self) -> Result<()> {
        if let Some(peeked_char) = self.input.chars().next() {
            self.eat(1);
            if peeked_char == '\'' {
                self.store_buf_as_token();
                self.state = LexerState::Control;
            } else {
                self.buf.push(peeked_char);
            }
            Ok(())
        } else {
            Err(Error::LexerError)
        }
    }

    fn scan_doublequote(&mut self) -> Result<()> {
        if let Some(peeked_char) = self.input.chars().next() {
            self.eat(1);
            if self.escaping {
                self.buf.push(peeked_char);
                self.escaping = false;
            } else if peeked_char == '`' {
                self.escaping = true;
            } else if peeked_char == '"' {
                self.store_buf_as_token();
                self.state = LexerState::Control;
            } else {
                self.buf.push(peeked_char);
            }
            Ok(())
        } else {
            Err(Error::LexerError)
        }
    }

    fn scan_parantheses_cmd(&mut self) -> Result<()> {
        if let Some(peeked_char) = self.input.chars().next() {
            self.eat(1);
            self.buf.push(peeked_char);
            if peeked_char == ')' {
                self.store_buf_as_token();
                self.state = LexerState::Control;
            }
            Ok(())
        } else {
            Err(Error::LexerError)
        }
    }

    fn scan_maybearrayop(&mut self) -> Result<()> {
        if let Some(peeked_char) = self.input.chars().next() {
            self.eat(1);
            if peeked_char == '(' {
                self.tokens.push(Token::ArrayOpBegin);
            } else {
                self.buf.push('@');
                self.buf.push(peeked_char);
            }
            self.state = LexerState::Control;
            Ok(())
        } else {
            Err(Error::LexerError)
        }
    }

    fn is_number(&self) -> bool {
        self.buf.parse::<f64>().is_ok()
    }

    fn is_bool(&self) -> Option<bool> {
        if self.buf == "$True" {
            Some(true)
        } else if self.buf == "$False" {
            Some(false)
        } else {
            None
        }
    }

    fn store_buf_as_token(&mut self) {
        if !self.buf.is_empty() {
            if self.is_number() {
                self.tokens
                    .push(Token::Number(std::mem::take(&mut self.buf)));
            } else if let Some(bool_value) = self.is_bool() {
                self.buf.clear();
                self.tokens.push(Token::Bool(bool_value));
            } else {
                self.tokens
                    .push(Token::String(std::mem::take(&mut self.buf)));
            }
        }
    }

    fn eat(&mut self, num: usize) {
        self.input = &self.input[num..];
    }
}

pub struct Parser<'a> {
    input: &'a [Token],
}

impl<'a> Parser<'a> {
    // argument : array
    //          | sequence_by_comma_op
    //          | SKALAR'''
    pub fn parse_argument(&mut self) -> Result<CliArgument> {
        self.parse_sequence_by_comma_op().or_else(|_| {
            self.parse_array()
                .or_else(|_| self.parse_skalar().or(Err(Error::ParserError)))
        })
    }

    // array : ARRAY_BEGIN sequence ARRAY_END
    //       | ARRAY_OP sequence PARANTHESES_CLOSE
    //       | ARRAY_OP PARANTHESES_CLOSE
    //       | ARRAY_BEGIN ARRAY_END
    fn parse_array(&mut self) -> Result<CliArgument> {
        let backtrack = self.input;
        /*if self.parse_array_empty().is_ok() {
            return Ok(CliArgument::Array(Vec::new()));
        }*/
        if self.parse_newtype_token(Token::ArrayBegin).is_ok() {
            let mut sequence_by_array = Vec::new();
            if let Ok(mut sequence) = self.parse_sequence() {
                sequence_by_array.append(&mut sequence);
            }
            if self.parse_newtype_token(Token::ArrayEnd).is_ok() {
                return Ok(CliArgument::Array(sequence_by_array));
            }
        } else if self.parse_newtype_token(Token::ArrayOpBegin).is_ok() {
            let mut sequence_by_array = Vec::new();
            if let Ok(mut sequence) = self.parse_sequence() {
                sequence_by_array.append(&mut sequence);
            }
            if self.parse_newtype_token(Token::ArrayOpEnd).is_ok() {
                return Ok(CliArgument::Array(sequence_by_array));
            }
        }
        self.input = backtrack;
        Err(Error::ParserError)
    }

    // comma_op : element COMMA
    fn parse_comma_op(&mut self) -> Result<Vec<CliArgument>> {
        let backtrack = self.input;
        if let Ok(element) = self.parse_element() {
            if self.parse_newtype_token(Token::Comma).is_ok() {
                return Ok(vec![element]);
            }
        }
        self.input = backtrack;
        Err(Error::ParserError)
    }

    // sequence_by_comma_op : comma_op
    //                      | comma_op sequence
    fn parse_sequence_by_comma_op(&mut self) -> Result<CliArgument> {
        let backtrack = self.input;
        if let Ok(mut sequence_by_comma_op) = self.parse_comma_op() {
            if let Ok(mut sequence) = self.parse_sequence() {
                sequence_by_comma_op.append(&mut sequence);
            }
            Ok(CliArgument::Array(sequence_by_comma_op))
        } else {
            self.input = backtrack;
            Err(Error::ParserError)
        }
    }

    // sequence : element
    //          | element COMMA sequence"""
    fn parse_sequence(&mut self) -> Result<Vec<CliArgument>> {
        let backtrack = self.input;
        if let Ok(element) = self.parse_element() {
            let mut sequence = vec![element];
            loop {
                if self.parse_newtype_token(Token::Comma).is_err() {
                    break;
                }
                if let Ok(element) = self.parse_element() {
                    sequence.push(element);
                } else {
                    break;
                }
            }
            Ok(sequence)
        } else {
            self.input = backtrack;
            Err(Error::ParserError)
        }
    }

    // element : skalar
    //         | array
    fn parse_element(&mut self) -> Result<CliArgument> {
        let backtrack = self.input;
        self.parse_skalar().or_else(|_| {
            self.parse_array().map_err(|_| {
                self.input = backtrack;
                Error::ParserError
            })
        })
    }

    fn parse_skalar(&mut self) -> Result<CliArgument> {
        let backtrack = self.input;
        if !self.input.is_empty() {
            if let Ok(skalar) = match self.input[0] {
                Token::String(ref string_token) => Ok(CliArgument::String(string_token.clone())),
                Token::Number(ref number_token) => {
                    Ok(CliArgument::Number(Number::parse(number_token).unwrap()))
                }
                Token::Bool(bool_token) => Ok(CliArgument::Bool(bool_token)),
                _ => Err(Error::ParserError),
            } {
                self.input = &self.input[1..];
                return Ok(skalar);
            }
        }
        self.input = backtrack;
        Err(Error::ParserError)
    }

    fn parse_newtype_token(&mut self, token: Token) -> Result<Token> {
        let backtrack = self.input;
        if !self.input.is_empty() && self.input[0] == token {
            self.input = &self.input[1..];
            return Ok(token);
        }
        self.input = backtrack;
        Err(Error::ParserError)
    }
}

#[cfg(test)]
mod test_number {
    use super::Number;

    #[test]
    fn test_number() {
        assert!(Number::parse("123") == Some(Number::PosInt(123)));
        assert!(Number::parse("-123") == Some(Number::NegInt(-123)));
        assert!(Number::parse("123.456") == Some(Number::Float(123.456)));
        assert!(Number::parse("-123.456") == Some(Number::Float(-123.456)));
        assert!(Number::parse("-+1") == None);
    }
}

#[cfg(test)]
mod test_lexer {
    use super::{Lexer, Token};

    #[test]
    fn test_lexer() {
        let input = "[]";
        let lexer = Lexer::from_str(input);
        assert!(lexer.lex().unwrap() == vec![Token::ArrayBegin, Token::ArrayEnd]);
        let input = "@()";
        let lexer = Lexer::from_str(input);
        assert!(lexer.lex().unwrap() == vec![Token::ArrayOpBegin, Token::ArrayOpEnd]);
        let input = "abc";
        let lexer = Lexer::from_str(input);
        assert!(lexer.lex().unwrap() == vec![Token::String("abc".to_owned())]);
        let input = "abc,123";
        let lexer = Lexer::from_str(input);
        assert!(
            lexer.lex().unwrap()
                == vec![
                    Token::String("abc".to_owned()),
                    Token::Comma,
                    Token::Number("123".to_owned())
                ]
        );
        let input = "$False,$True";
        let lexer = Lexer::from_str(input);
        assert!(lexer.lex().unwrap() == vec![Token::Bool(false), Token::Comma, Token::Bool(true)]);
        let input = "[foo,123]";
        let lexer = Lexer::from_str(input);
        assert!(
            lexer.lex().unwrap()
                == vec![
                    Token::ArrayBegin,
                    Token::String("foo".to_owned()),
                    Token::Comma,
                    Token::Number("123".to_owned()),
                    Token::ArrayEnd
                ]
        );
        let input = r#"@("foo",123)"#;
        let lexer = Lexer::from_str(input);
        assert!(
            lexer.lex().unwrap()
                == vec![
                    Token::ArrayOpBegin,
                    Token::String("foo".to_owned()),
                    Token::Comma,
                    Token::Number("123".to_owned()),
                    Token::ArrayOpEnd
                ]
        );
        let input = r#""abc,123" , 'def,456'"#;
        let lexer = Lexer::from_str(input);
        assert!(
            lexer.lex().unwrap()
                == vec![
                    Token::String("abc,123".to_owned()),
                    Token::Comma,
                    Token::String("def,456".to_owned())
                ]
        );
        let input = r#"`"`'```[`]"#;
        let lexer = Lexer::from_str(input);
        assert!(lexer.lex().unwrap() == vec![Token::String(r#""'`[]"#.to_owned())]);
    }
}

#[cfg(test)]
mod test_parser {
    use super::{CliArgument, Number, Parser, Token};

    #[test]
    fn test_skalars() {
        let tokens = vec![Token::Bool(true)];
        let mut parser = Parser { input: &tokens };
        let result = parser.parse_argument().unwrap();
        let expected = CliArgument::Bool(true);
        assert!(result == expected);

        let tokens = vec![Token::Number("123".to_owned())];
        let mut parser = Parser { input: &tokens };
        let result = parser.parse_argument().unwrap();
        let expected = CliArgument::Number(Number::PosInt(123));
        assert!(result == expected);

        let tokens = vec![Token::String("Hello World".to_owned())];
        let mut parser = Parser { input: &tokens };
        let result = parser.parse_argument().unwrap();
        let expected = CliArgument::String("Hello World".to_owned());
        assert!(result == expected);
    }

    #[test]
    fn test_array_by_comma_op() {
        let tokens = vec![
            Token::Bool(true),
            Token::Comma,
            Token::String("Hello World".to_owned()),
            Token::Comma,
            Token::Number("123".to_owned()),
        ];
        let mut parser = Parser { input: &tokens };
        let result = parser.parse_argument().unwrap();
        let expected = CliArgument::Array(vec![
            CliArgument::Bool(true),
            CliArgument::String("Hello World".to_owned()),
            CliArgument::Number(Number::PosInt(123)),
        ]);
        assert!(result == expected);
    }

    #[test]
    fn test_array_1bool() {
        let tokens = vec![Token::ArrayBegin, Token::Bool(true), Token::ArrayEnd];
        let mut parser = Parser { input: &tokens };
        let result = parser.parse_array().unwrap();
        let expected = CliArgument::Array(vec![CliArgument::Bool(true)]);
        assert!(result == expected);
    }

    #[test]
    fn test_bool_array_of_array_1bool() {
        let tokens = vec![
            Token::ArrayBegin,
            Token::ArrayBegin,
            Token::Bool(true),
            Token::ArrayEnd,
            Token::ArrayEnd,
        ];
        let mut parser = Parser { input: &tokens };
        let result = parser.parse_array().unwrap();
        let expected = CliArgument::Array(vec![CliArgument::Array(vec![CliArgument::Bool(true)])]);
        assert!(result == expected);
    }
}

#[cfg(test)]
mod test_parser_and_lexer {
    use super::{from_str, CliArgument, Number};
    use serde_json;

    #[test]
    fn test_example1() {
        let input = r#"foo"#;
        let result = from_str(input).unwrap();
        let expected = CliArgument::String("foo".to_owned());
        assert!(result == expected);
        assert!(serde_json::to_string(&result).unwrap() == r#""foo""#);
    }

    #[test]
    fn test_example2() {
        let input = r#""foo""#;
        let result = from_str(input).unwrap();
        let expected = CliArgument::String("foo".to_owned());
        assert!(result == expected);
        assert!(serde_json::to_string(&result).unwrap() == r#""foo""#);
    }

    #[test]
    fn test_example3() {
        let input = r#"123"#;
        let result = from_str(input).unwrap();
        let expected = CliArgument::Number(Number::PosInt(123));
        assert!(result == expected);
        assert!(serde_json::to_string(&result).unwrap() == r#"123"#);
    }

    #[test]
    fn test_example4() {
        let input = r#"foo,123"#;
        let result = from_str(input).unwrap();
        let expected = CliArgument::Array(vec![
            CliArgument::String("foo".to_owned()),
            CliArgument::Number(Number::PosInt(123)),
        ]);
        assert!(result == expected);
        assert!(serde_json::to_string(&result).unwrap() == r#"["foo",123]"#);
    }

    #[test]
    fn test_example5() {
        let input = r#""foo,123""#;
        let result = from_str(input).unwrap();
        let expected = CliArgument::String("foo,123".to_owned());
        assert!(result == expected);
        assert!(serde_json::to_string(&result).unwrap() == r#""foo,123""#);
    }

    #[test]
    fn test_example6() {
        let input = r#"["foo",123]"#;
        let result = from_str(input).unwrap();
        let expected = CliArgument::Array(vec![
            CliArgument::String("foo".to_owned()),
            CliArgument::Number(Number::PosInt(123)),
        ]);
        assert!(result == expected);
        assert!(serde_json::to_string(&result).unwrap() == r#"["foo",123]"#);
    }

    #[test]
    fn test_example7() {
        let input = r#"@("foo",123)"#;
        let result = from_str(input).unwrap();
        let expected = CliArgument::Array(vec![
            CliArgument::String("foo".to_owned()),
            CliArgument::Number(Number::PosInt(123)),
        ]);
        assert!(result == expected);
        assert!(serde_json::to_string(&result).unwrap() == r#"["foo",123]"#);
    }

    #[test]
    fn test_example8() {
        let input = r#"[ foo , [ 123 , 456 ] ]"#;
        let result = from_str(input).unwrap();
        let expected = CliArgument::Array(vec![
            CliArgument::String("foo".to_owned()),
            CliArgument::Array(vec![
                CliArgument::Number(Number::PosInt(123)),
                CliArgument::Number(Number::PosInt(456)),
            ]),
        ]);
        assert!(result == expected);
        assert!(serde_json::to_string(&result).unwrap() == r#"["foo",[123,456]]"#);
    }

    #[test]
    fn test_example9() {
        let input = r#"$False,$True"#;
        let result = from_str(input).unwrap();
        let expected = CliArgument::Array(vec![CliArgument::Bool(false), CliArgument::Bool(true)]);
        assert!(result == expected);
        assert!(serde_json::to_string(&result).unwrap() == r#"[false,true]"#);
    }

    #[test]
    fn test_example10() {
        let input = r#"'"hello, world"'"#;
        let result = from_str(input).unwrap();
        let expected = CliArgument::String("\"hello, world\"".to_owned());
        assert!(result == expected);
        assert!(serde_json::to_string(&result).unwrap() == r#""\"hello, world\"""#);
    }

    #[test]
    fn test_example11() {
        let input = r#""literal `" doublequote""#;
        let result = from_str(input).unwrap();
        let expected = CliArgument::String("literal \" doublequote".to_owned());
        assert!(result == expected);
        assert!(serde_json::to_string(&result).unwrap() == r#""literal \" doublequote""#);
    }

    #[test]
    fn test_example12() {
        let input = r#"(ConvertTo-IcingaSecureString 'my string')"#;
        let result = from_str(input).unwrap();
        let expected = CliArgument::String("(ConvertTo-IcingaSecureString 'my string')".to_owned());
        assert!(result == expected);
        assert!(
            serde_json::to_string(&result).unwrap()
                == r#""(ConvertTo-IcingaSecureString 'my string')""#
        );
    }
}
