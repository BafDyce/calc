#![cfg_attr(test, feature(test))]

#[cfg(test)]
extern crate test;

use self::CalcError::*;

use std::error::Error;
use std::fmt;
use std::io;
use std::iter::Peekable;
use std::num::ParseFloatError;

#[cfg(test)]
mod bench;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Plus,
    Minus,
    Divide,
    Multiply,
    Exponent,
    Square,
    Cube,
    BitWiseAnd,
    BitWiseOr,
    BitWiseXor,
    BitWiseNot,
    BitWiseRShift,
    BitWiseLShift,
    Modulo,
    OpenParen,
    CloseParen,
    Number(f64),
}


impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tok = match *self {
            Token::Plus => "Plus",
            Token::Minus => "Minus",
            Token::Divide => "Divide",
            Token::Multiply => "Multiply",
            Token::Exponent => "Exponent",
            Token::Square => "Square",
            Token::Cube => "Cube",
            Token::BitWiseAnd => "And",
            Token::BitWiseOr => "Or",
            Token::BitWiseXor => "Xor",
            Token::BitWiseNot => "Not",
            Token::BitWiseRShift => "RShift",
            Token::BitWiseLShift => "LShift",
            Token::Modulo => "Modulo",
            Token::OpenParen => "OpenParen",
            Token::CloseParen => "CloseParen",
            Token::Number(_) => "Number",
        };
        write!(f, "{}", tok)
    }
}

#[derive(Debug)]
pub enum CalcError {
    DivideByZero,
    InvalidNumber(String),
    InvalidOperator(char),
    UnrecognizedToken(String),
    UnexpectedToken(String, &'static str),
    UnexpectedEndOfInput,
    UnmatchedParenthesis,
    IO(io::Error),
}

impl From<CalcError> for String {
    fn from(data: CalcError) -> String {
        match data {
            DivideByZero => String::from("calc: attempted to divide by zero"),
            InvalidNumber(number) => ["calc: invalid number: ", &number].concat(),
            InvalidOperator(character) => format!("calc: invalid operator: {}", character),
            IO(error) => error.description().to_owned(),
            UnrecognizedToken(token) => ["calc: unrecognized token: ", &token].concat(),
            UnexpectedToken(token, kind) => {
                ["calc: unexpected ", kind, " token: ", &token].concat()
            }
            UnexpectedEndOfInput => String::from("calc: unexpected end of input"),
            UnmatchedParenthesis => String::from("calc: unmatched parenthesis"),
        }
    }
}

// By implementing this, we can have Rust automatically cast io::Errors into
// calc errors, which reduces noise
impl From<io::Error> for CalcError {
    fn from(data: io::Error) -> CalcError {
        CalcError::IO(data)
    }
}

impl From<ParseFloatError> for CalcError {
    fn from(data: ParseFloatError) -> CalcError {
        CalcError::InvalidNumber(data.description().into())
    }
}

#[derive(Clone, Debug)]
pub struct IntermediateResult {
    value: f64,
    tokens_read: usize,
}

impl IntermediateResult {

    fn new(value: f64, tokens_read: usize) -> Self {
        IntermediateResult { value, tokens_read }
    }

    /// Determines if the underlying value can be represented as an integer. This is used for
    /// typechecking of sorts: we can only do bitwise operations on integers.
    pub fn is_whole(&self) -> bool {
        self.value == self.value.floor()
    }

}

enum OperatorState {
    PotentiallyIncomplete,
    Complete,
    NotAnOperator,
}

trait IsOperator {
    fn is_operator(self) -> bool;
}

impl IsOperator for char {
    fn is_operator(self) -> bool {
        match self {
            '+' | '-' | '/' | '^' | '²' | '³' | '&' | '|' | '~' | '>' | '%' | '(' | ')' |
            '*' | '<' => true,
            _ => false,
        }
    }
}

trait CheckOperator {
    fn check_operator(self) -> OperatorState;
}

impl CheckOperator for char {
    fn check_operator(self) -> OperatorState {
        match self {
            '+' | '-' | '/' | '^' | '²' | '³' | '&' | '|' | '~' | '%' | '(' | ')' => {
                OperatorState::Complete
            }
            '*' | '<' | '>' => OperatorState::PotentiallyIncomplete,
            _ => OperatorState::NotAnOperator,
        }
    }
}

pub trait OperatorMatch {
    fn operator_type(self) -> Option<Token>;
}

impl OperatorMatch for [char; 2] {
    fn operator_type(self) -> Option<Token> {
        if self == ['*', '*'] {
            Some(Token::Exponent)
        } else if self == ['<', '<'] {
            Some(Token::BitWiseLShift)
        } else if self == ['>', '>'] {
            Some(Token::BitWiseRShift)
        } else {
            None
        }
    }
}

impl OperatorMatch for char {
    fn operator_type(self) -> Option<Token> {
        match self {
            '+' => Some(Token::Plus),
            '-' => Some(Token::Minus),
            '/' => Some(Token::Divide),
            '*' => Some(Token::Multiply),
            '^' => Some(Token::BitWiseXor),
            '²' => Some(Token::Square),
            '³' => Some(Token::Cube),
            '&' => Some(Token::BitWiseAnd),
            '|' => Some(Token::BitWiseOr),
            '~' => Some(Token::BitWiseNot),
            '%' => Some(Token::Modulo),
            '(' => Some(Token::OpenParen),
            ')' => Some(Token::CloseParen),
            _ => None,
        }
    }
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, CalcError> {
    let mut tokens = Vec::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c.is_digit(10) || c == '.' {
            let token_string = consume_number(&mut chars);
            tokens.push(Token::Number(token_string.parse()?));
        } else {
            match c.check_operator() {
                OperatorState::Complete => {
                    tokens.push(c.operator_type().ok_or_else(|| InvalidOperator(c))?);
                    chars.next();
                }
                OperatorState::PotentiallyIncomplete => {
                    chars.next();
                    match chars.peek() {
                        Some(&next_char) if next_char.is_operator() => {
                            tokens.push([c, next_char].operator_type().ok_or_else(
                                || InvalidOperator(c),
                            )?);
                            chars.next();
                        }
                        _ => {
                            tokens.push(c.operator_type().ok_or_else(|| InvalidOperator(c))?);
                        }
                    }
                }
                OperatorState::NotAnOperator => {
                    if c.is_whitespace() {
                        chars.next();
                    } else {
                        let token_string = consume_until_new_token(&mut chars);
                        return Err(CalcError::UnrecognizedToken(token_string));
                    }
                }
            }
        }
    }
    Ok(tokens)
}

fn consume_number<I: Iterator<Item = char>>(input: &mut Peekable<I>) -> String {
    let mut number = String::new();
    let mut has_decimal_point = false;
    while let Some(&c) = input.peek() {
        if c == '.' {
            if has_decimal_point {
                break;
            } else {
                number.push(c);
                has_decimal_point = true;
            }
        } else if c.is_digit(10) {
            number.push(c);
        } else {
            break;
        }
        input.next();
    }
    number
}

fn consume_until_new_token<I: Iterator<Item = char>>(input: &mut I) -> String {
    input
        .take_while(|c| {
            !(c.is_whitespace() || c.is_operator() || c.is_digit(10))
        })
        .collect()
}

pub fn d_expr(token_list: &[Token]) -> Result<IntermediateResult, CalcError> {
    let mut e1 = e_expr(token_list)?;
    let mut index = e1.tokens_read;

    while index < token_list.len() {
        match token_list[index] {
            Token::BitWiseAnd => {
                let e2 = e_expr(&token_list[index + 1..])?;
                if e1.is_whole() && e2.is_whole() {
                    let mut int_f = e1.value.floor() as i64;
                    let int_s = e2.value.floor() as i64;
                    int_f &= int_s;
                    e1.value = int_f as f64;
                    e1.tokens_read += e2.tokens_read + 1;
                } else {
                    return Err(CalcError::UnexpectedToken(
                        (if e1.is_whole() { e2.value } else { e1.value }).to_string(),
                        "Not a integer number!",
                    ));
                }
            }
            Token::BitWiseOr => {
                let e2 = e_expr(&token_list[index + 1..])?;
                if e1.is_whole() && e2.is_whole() {
                    let mut int_f = e1.value.floor() as i64;
                    let int_s = e2.value.floor() as i64;
                    int_f |= int_s;
                    e1.value = int_f as f64;
                    e1.tokens_read += e2.tokens_read + 1;
                } else {
                    return Err(CalcError::UnexpectedToken(
                        (if e1.is_whole() { e2.value } else { e1.value }).to_string(),
                        "Not a integer number!",
                    ));
                }
            }
            Token::BitWiseNot => {
                if e1.is_whole() {
                    let mut int_f = e1.value.floor() as i64;
                    //magic number: bigest integer representable by f64 is 2^53, which is 0b1<<54 according to https://stackoverflow.com/questions/1848700/biggest-integer-that-can-be-stored-in-a-double
                    // make a mask by shifting 11... between the sign bit and the number to effectively get a 55 bit signed number
                    //let mask = 0b111111111 << 54;
                    int_f = !(int_f);
                    e1.value = int_f as f64;
                    e1.tokens_read += 1;
                } else {
                    return Err(CalcError::UnexpectedToken(e1.value.to_string(), "Not a integer number!"));
                }
            }
            Token::BitWiseXor => {
                let e2 = e_expr(&token_list[index + 1..])?;
                if e1.is_whole() && e2.is_whole() {
                    let mut int_f = e1.value.floor() as i64;
                    let int_s = e2.value.floor() as i64;
                    int_f ^= int_s;
                    e1.value = int_f as f64;
                    e1.tokens_read += e2.tokens_read + 1;
                } else {
                    return Err(CalcError::UnexpectedToken(
                        (if e1.is_whole() { e2.value } else { e1.value }).to_string(),
                        "Not a integer number!",
                    ));
                }
            }
            Token::BitWiseLShift => {
                let e2 = e_expr(&token_list[index + 1..])?;
                if e1.is_whole() && e2.is_whole() {
                    let mut int_f = e1.value.floor() as i64;
                    let int_s = e2.value.floor() as i64;
                    int_f <<= int_s;
                    e1.value = int_f as f64;
                    e1.tokens_read += e2.tokens_read + 1;
                } else {
                    return Err(CalcError::UnexpectedToken(
                        (if e1.is_whole() { e2.value } else { e1.value }).to_string(),
                        "Not a integer number!",
                    ));
                }
            }
            Token::BitWiseRShift => {
                let e2 = e_expr(&token_list[index + 1..])?;
                if e1.is_whole() && e2.is_whole() {
                    let mut int_f = e1.value.floor() as i64;
                    let int_s = e2.value.floor() as i64;
                    int_f >>= int_s;
                    e1.value = int_f as f64;
                    e1.tokens_read += e2.tokens_read + 1;
                } else {
                    return Err(CalcError::UnexpectedToken(
                        (if e1.is_whole() { e2.value } else { e1.value }).to_string(),
                        "Not a integer number!",
                    ));
                }
            }
            Token::Number(ref n) => {
                return Err(CalcError::UnexpectedToken(n.to_string(), "operator"));
            }
            _ => break,
        };
        index = e1.tokens_read;
    }
    Ok(e1)
}
// Addition and subtraction
pub fn e_expr(token_list: &[Token]) -> Result<IntermediateResult, CalcError> {
    let mut t1 = t_expr(token_list)?;
    let mut index = t1.tokens_read;

    while index < token_list.len() {
        match token_list[index] {
            Token::Plus => {
                let t2 = t_expr(&token_list[index + 1..])?;
                t1.value += t2.value;
                t1.tokens_read += t2.tokens_read + 1;
            }
            Token::Minus => {
                let t2 = t_expr(&token_list[index + 1..])?;
                t1.value -= t2.value;
                t1.tokens_read += t2.tokens_read + 1;
            }
            Token::Number(n) => return Err(CalcError::UnexpectedToken(n.to_string(), "operator")),
            _ => break,
        };
        index = t1.tokens_read;
    }
    Ok(t1)
}

// Multiplication and division
pub fn t_expr(token_list: &[Token]) -> Result<IntermediateResult, CalcError> {
    let mut f1 = f_expr(token_list)?;
    let mut index = f1.tokens_read;

    while index < token_list.len() {
        match token_list[index] {
            Token::Multiply => {
                let f2 = f_expr(&token_list[index + 1..])?;
                f1.value *= f2.value;
                f1.tokens_read += f2.tokens_read + 1;
            }
            Token::Divide => {
                let f2 = f_expr(&token_list[index + 1..])?;
                if f2.value == 0.0 {
                    return Err(CalcError::DivideByZero);
                } else {
                    f1.value /= f2.value;
                    f1.tokens_read += f2.tokens_read + 1;
                }
            }
            Token::Modulo => {
                let f2 = f_expr(&token_list[index + 1..])?;
                if f2.value == 0.0 {
                    return Err(CalcError::DivideByZero);
                } else {
                    f1.value %= f2.value;
                    f1.tokens_read += f2.tokens_read + 1;
                }
            }
            Token::Number(n) => {
                return Err(CalcError::UnexpectedToken(n.to_string(), "operator"));
            }
            _ => break,
        }
        index = f1.tokens_read;
    }
    Ok(f1)
}

// Exponentiation
pub fn f_expr(token_list: &[Token]) -> Result<IntermediateResult, CalcError> {
    let mut g1 = g_expr(token_list)?; //was g1
    let mut index = g1.tokens_read;
    let token_len = token_list.len();
    while index < token_len {
        match token_list[index] {
            Token::Exponent => {
                let f = f_expr(&token_list[index + 1..])?;
                g1.value = g1.value.powf(f.value);
                g1.tokens_read += f.tokens_read + 1;
            }
            Token::Square => {
                g1.value = g1.value * g1.value;
                g1.tokens_read += 1;
            }
            Token::Cube => {
                g1.value = g1.value * g1.value * g1.value;
                g1.tokens_read += 1;
            }
            Token::Number(n) => {
                return Err(CalcError::UnexpectedToken(n.to_string(), "operator"));
            }
            _ => break,
        }
        index = g1.tokens_read;
    }
    Ok(g1)
}

// Numbers and parenthesized expressions
pub fn g_expr(token_list: &[Token]) -> Result<IntermediateResult, CalcError> {
    if !token_list.is_empty() {
        match token_list[0] {
            Token::Number(n) => Ok(IntermediateResult::new(n, 1)),
            Token::Minus => {
                if token_list.len() > 1 {
                    if let Token::Number(ref n) = token_list[1] {
                        Ok(IntermediateResult::new(-n, 2))
                    } else {
                        Err(CalcError::UnexpectedToken(
                            token_list[1].to_string(),
                            "number",
                        ))
                    }
                } else {
                    Err(CalcError::UnexpectedEndOfInput)
                }
            }
            Token::OpenParen => {
                let expr = d_expr(&token_list[1..]);
                match expr {
                    Ok(ir) => {
                        let close_paren = ir.tokens_read + 1;
                        if close_paren < token_list.len() {
                            match token_list[close_paren] {
                                Token::CloseParen => Ok(IntermediateResult::new(
                                    ir.value,
                                    close_paren + 1,
                                )),
                                _ => Err(CalcError::UnexpectedToken(
                                    token_list[close_paren].to_string(),
                                    ")",
                                )),
                            }
                        } else {
                            Err(CalcError::UnmatchedParenthesis)
                        }
                    }
                    Err(e) => Err(e),
                }
            }
            _ => Err(CalcError::UnexpectedToken(
                token_list[0].to_string(),
                "number",
            )),
        }
    } else {
        Err(CalcError::UnexpectedEndOfInput)
    }
}


pub fn parse(tokens: &[Token]) -> Result<f64, CalcError> {
    d_expr(tokens).map(|answer| answer.value)
}

pub fn eval(input: &str) -> Result<f64, CalcError> {
    tokenize(input).and_then(|x| parse(&x))
}
