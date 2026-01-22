//! Parser: DQL to AST
//!
//! Hand-written recursive descent parser. No parser generators.

use crate::ast::{Filter, FuncName, Function, Mutation, Operation, Order, Query, Value};
use dgraph_common::Uid;
use thiserror::Error;

/// Parse error
#[derive(Error, Debug, Clone, PartialEq)]
#[error("parse error at position {pos}: {msg}")]
pub struct ParseError {
    /// Position in input
    pub pos: usize,
    /// Error message
    pub msg: String,
}

impl ParseError {
    fn new(pos: usize, msg: impl Into<String>) -> Self {
        Self { pos, msg: msg.into() }
    }
}

/// DQL Parser
pub struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    /// Create new parser
    #[must_use]
    pub const fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    /// Parse a complete operation
    ///
    /// # Errors
    /// Returns error on invalid syntax
    pub fn parse(&mut self) -> Result<Operation, ParseError> {
        self.skip_whitespace();

        // Check for mutation
        if self.peek_str("mutation") {
            return self.parse_mutation().map(Operation::Mutation);
        }

        // Otherwise it's a query
        self.parse_query_block().map(Operation::Query)
    }

    /// Parse a query block: { ... }
    fn parse_query_block(&mut self) -> Result<Query, ParseError> {
        self.skip_whitespace();
        self.expect('{')?;

        let query = self.parse_query_inner()?;

        self.skip_whitespace();
        self.expect('}')?;

        Ok(query)
    }

    /// Parse inner query content
    fn parse_query_inner(&mut self) -> Result<Query, ParseError> {
        self.skip_whitespace();

        let mut query = Query::default();

        // Parse alias or predicate name
        let name = self.parse_identifier()?;

        self.skip_whitespace();

        // Check for root function: name(func: ...)
        if self.peek() == Some('(') {
            self.advance();
            self.skip_whitespace();

            // Parse func: funcname(args)
            if self.peek_str("func:") {
                self.advance_by(5);
                self.skip_whitespace();
                query.func = Some(self.parse_function()?);
            }

            // Parse directives: first, offset, orderasc, orderdesc
            loop {
                self.skip_whitespace();
                if self.peek() == Some(')') {
                    break;
                }
                if self.peek() == Some(',') {
                    self.advance();
                    self.skip_whitespace();
                }

                if self.peek_str("first:") {
                    self.advance_by(6);
                    self.skip_whitespace();
                    query.first = Some(self.parse_int()? as u32);
                } else if self.peek_str("offset:") {
                    self.advance_by(7);
                    self.skip_whitespace();
                    query.offset = Some(self.parse_int()? as u32);
                } else if self.peek_str("orderasc:") {
                    self.advance_by(9);
                    self.skip_whitespace();
                    let attr = self.parse_identifier()?;
                    query.order = Some(Order { attr, desc: false });
                } else if self.peek_str("orderdesc:") {
                    self.advance_by(10);
                    self.skip_whitespace();
                    let attr = self.parse_identifier()?;
                    query.order = Some(Order { attr, desc: true });
                } else if self.peek() != Some(')') {
                    return Err(self.error("unexpected token in query parameters"));
                }
            }

            self.expect(')')?;
            query.alias = Some(name);
        } else {
            query.attr = Some(name);
        }

        self.skip_whitespace();

        // Parse @filter directive
        if self.peek_str("@filter") {
            self.advance_by(7);
            self.skip_whitespace();
            self.expect('(')?;
            query.filter = Some(self.parse_filter()?);
            self.expect(')')?;
            self.skip_whitespace();
        }

        // Parse children if there's a block
        if self.peek() == Some('{') {
            self.advance();
            query.children = self.parse_children()?;
            self.skip_whitespace();
            self.expect('}')?;
        }

        Ok(query)
    }

    /// Parse child queries
    fn parse_children(&mut self) -> Result<Vec<Query>, ParseError> {
        let mut children = Vec::new();

        loop {
            self.skip_whitespace();
            if self.peek() == Some('}') || self.is_eof() {
                break;
            }

            let child = self.parse_query_inner()?;
            children.push(child);
        }

        Ok(children)
    }

    /// Parse a function: funcname(args)
    fn parse_function(&mut self) -> Result<Function, ParseError> {
        let name_str = self.parse_identifier()?;
        let name = FuncName::from_str(&name_str)
            .ok_or_else(|| self.error(format!("unknown function: {name_str}")))?;

        self.skip_whitespace();
        self.expect('(')?;

        let mut args = Vec::new();
        loop {
            self.skip_whitespace();
            if self.peek() == Some(')') {
                break;
            }
            if !args.is_empty() {
                self.expect(',')?;
                self.skip_whitespace();
            }
            args.push(self.parse_value()?);
        }

        self.expect(')')?;

        Ok(Function { name, args })
    }

    /// Parse a filter expression
    fn parse_filter(&mut self) -> Result<Filter, ParseError> {
        self.parse_filter_or()
    }

    fn parse_filter_or(&mut self) -> Result<Filter, ParseError> {
        let mut left = self.parse_filter_and()?;

        loop {
            self.skip_whitespace();
            if !self.peek_str("OR") && !self.peek_str("or") {
                break;
            }
            self.advance_by(2);
            self.skip_whitespace();
            let right = self.parse_filter_and()?;
            left = Filter::Or(vec![left, right]);
        }

        Ok(left)
    }

    fn parse_filter_and(&mut self) -> Result<Filter, ParseError> {
        let mut left = self.parse_filter_not()?;

        loop {
            self.skip_whitespace();
            if !self.peek_str("AND") && !self.peek_str("and") {
                break;
            }
            self.advance_by(3);
            self.skip_whitespace();
            let right = self.parse_filter_not()?;
            left = Filter::And(vec![left, right]);
        }

        Ok(left)
    }

    fn parse_filter_not(&mut self) -> Result<Filter, ParseError> {
        self.skip_whitespace();
        if self.peek_str("NOT") || self.peek_str("not") {
            self.advance_by(3);
            self.skip_whitespace();
            let inner = self.parse_filter_primary()?;
            return Ok(Filter::Not(Box::new(inner)));
        }
        self.parse_filter_primary()
    }

    fn parse_filter_primary(&mut self) -> Result<Filter, ParseError> {
        self.skip_whitespace();

        if self.peek() == Some('(') {
            self.advance();
            let inner = self.parse_filter()?;
            self.skip_whitespace();
            self.expect(')')?;
            return Ok(inner);
        }

        // Must be a function
        let func = self.parse_function()?;
        Ok(Filter::Func(func))
    }

    /// Parse a value
    fn parse_value(&mut self) -> Result<Value, ParseError> {
        self.skip_whitespace();

        match self.peek() {
            Some('"') => {
                let s = self.parse_string()?;
                Ok(Value::String(s))
            }
            Some('0'..='9') | Some('-') => {
                let start = self.pos;
                if self.peek() == Some('-') {
                    self.advance();
                }
                while matches!(self.peek(), Some('0'..='9' | '.')) {
                    self.advance();
                }
                let s = &self.input[start..self.pos];
                if s.contains('.') {
                    let f: f64 = s.parse().map_err(|_| self.error("invalid float"))?;
                    Ok(Value::Float(f))
                } else {
                    let i: i64 = s.parse().map_err(|_| self.error("invalid int"))?;
                    Ok(Value::Int(i))
                }
            }
            Some('$') => {
                self.advance();
                let name = self.parse_identifier()?;
                Ok(Value::Var(name))
            }
            Some('t') if self.peek_str("true") => {
                self.advance_by(4);
                Ok(Value::Bool(true))
            }
            Some('f') if self.peek_str("false") => {
                self.advance_by(5);
                Ok(Value::Bool(false))
            }
            Some('0') if self.peek_str("0x") => {
                self.advance_by(2);
                let start = self.pos;
                while matches!(self.peek(), Some('0'..='9' | 'a'..='f' | 'A'..='F')) {
                    self.advance();
                }
                let hex = &self.input[start..self.pos];
                let uid = u64::from_str_radix(hex, 16)
                    .map_err(|_| self.error("invalid hex uid"))?;
                Ok(Value::Uid(uid))
            }
            Some(c) if c.is_alphabetic() => {
                // Could be a predicate name as argument
                let s = self.parse_identifier()?;
                Ok(Value::String(s))
            }
            _ => Err(self.error("expected value")),
        }
    }

    /// Parse mutation block
    fn parse_mutation(&mut self) -> Result<Mutation, ParseError> {
        self.advance_by(8); // "mutation"
        self.skip_whitespace();
        self.expect('{')?;

        let mut mutation = Mutation::default();

        loop {
            self.skip_whitespace();
            if self.peek() == Some('}') {
                break;
            }

            if self.peek_str("set") {
                self.advance_by(3);
                self.skip_whitespace();
                self.expect('{')?;
                mutation.set = self.parse_triples()?;
                self.expect('}')?;
            } else if self.peek_str("delete") {
                self.advance_by(6);
                self.skip_whitespace();
                self.expect('{')?;
                mutation.delete = self.parse_triples()?;
                self.expect('}')?;
            } else {
                return Err(self.error("expected 'set' or 'delete'"));
            }
        }

        self.expect('}')?;
        Ok(mutation)
    }

    fn parse_triples(&mut self) -> Result<Vec<crate::ast::Triple>, ParseError> {
        // Simplified: just return empty for now
        // Full implementation would parse RDF triples
        self.skip_whitespace();
        Ok(Vec::new())
    }

    // --- Helper methods ---

    fn parse_identifier(&mut self) -> Result<String, ParseError> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_' || c == '.') {
            self.advance();
        }
        if self.pos == start {
            return Err(self.error("expected identifier"));
        }
        Ok(self.input[start..self.pos].to_string())
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        self.expect('"')?;
        let start = self.pos;
        while self.peek() != Some('"') && !self.is_eof() {
            if self.peek() == Some('\\') {
                self.advance(); // skip escape
            }
            self.advance();
        }
        let s = self.input[start..self.pos].to_string();
        self.expect('"')?;
        Ok(s)
    }

    fn parse_int(&mut self) -> Result<i64, ParseError> {
        let start = self.pos;
        if self.peek() == Some('-') {
            self.advance();
        }
        while matches!(self.peek(), Some('0'..='9')) {
            self.advance();
        }
        self.input[start..self.pos]
            .parse()
            .map_err(|_| self.error("invalid integer"))
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.advance();
        }
        // Skip comments
        if self.peek() == Some('#') {
            while self.peek() != Some('\n') && !self.is_eof() {
                self.advance();
            }
            self.skip_whitespace();
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn peek_str(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    fn advance(&mut self) {
        if let Some(c) = self.peek() {
            self.pos += c.len_utf8();
        }
    }

    fn advance_by(&mut self, n: usize) {
        for _ in 0..n {
            self.advance();
        }
    }

    fn expect(&mut self, c: char) -> Result<(), ParseError> {
        self.skip_whitespace();
        if self.peek() == Some(c) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(format!("expected '{c}'")))
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn error(&self, msg: impl Into<String>) -> ParseError {
        ParseError::new(self.pos, msg)
    }
}
