//! Parser: DQL to AST
//!
//! Hand-written recursive descent parser. No parser generators.
//! Supports variables, directives, and all DQL functions.

use crate::ast::{
    FacetOrder, FacetParam, FacetParams, Filter, FuncName, Function, GroupByAttr, MathExpr,
    MathOp, Mutation, Object, Operation, Order, Query, RecurseArgs, ShortestPathArgs, Subject,
    Triple, Value, VarContext, VarType,
};
use std::collections::HashMap;
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
        Self {
            pos,
            msg: msg.into(),
        }
    }
}

/// DQL Parser
pub struct Parser<'a> {
    input: &'a str,
    pos: usize,
    /// Variables defined in this query
    variables: HashMap<String, VarType>,
}

impl<'a> Parser<'a> {
    /// Create new parser
    #[must_use]
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            variables: HashMap::new(),
        }
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

        // Handle empty query
        if self.peek() == Some('}') {
            query.is_empty = true;
            return Ok(query);
        }

        // Check for variable definition: "x as name" or "x as pred(func: ...)"
        let _start_pos = self.pos;
        let first_ident = self.parse_identifier()?;
        self.skip_whitespace();

        if self.peek_str("as ") || self.peek_str("as\t") || self.peek_str("as\n") {
            // Variable definition
            self.advance_by(2); // skip "as"
            self.skip_whitespace();

            query.var = Some(first_ident.clone());
            self.variables.insert(first_ident, VarType::Uid);

            // Parse the actual query name
            let name = self.parse_identifier()?;
            self.skip_whitespace();

            if self.peek() == Some('(') {
                self.parse_query_params(&mut query)?;
            }
            query.alias = Some(name);
        } else if self.peek() == Some('(') {
            // Check if this is an aggregation function like count(predicate)
            if Self::is_aggregation_function(&first_ident) {
                self.advance(); // skip (
                self.skip_whitespace();
                let pred = self.parse_identifier()?;
                self.skip_whitespace();
                self.expect(')')?;
                query.attr = Some(pred);
                query.is_count = first_ident == "count";
                // For other aggregations, we still mark is_count as it's used for aggregation
                if first_ident == "sum" || first_ident == "avg" || first_ident == "min" || first_ident == "max" {
                    query.is_count = true;
                }
            } else {
                // Root query with function
                self.parse_query_params(&mut query)?;
                query.alias = Some(first_ident);
            }
        } else {
            // Simple predicate
            query.attr = Some(first_ident);
        }

        self.skip_whitespace();

        // Parse language specification: name@en or name@*
        if query.attr.is_some() && self.peek() == Some('@') && !self.peek_str("@filter") && !self.peek_str("@normalize") {
            self.advance(); // skip @
            let lang = self.parse_identifier()?;
            query.langs.push(lang);

            // Check for more languages: name@en:es:*
            while self.peek() == Some(':') {
                self.advance();
                let lang = self.parse_identifier()?;
                query.langs.push(lang);
            }
        }

        self.skip_whitespace();

        // Parse directives
        self.parse_directives(&mut query)?;

        // Parse children if there's a block
        if self.peek() == Some('{') {
            self.advance();
            query.children = self.parse_children()?;
            self.skip_whitespace();
            self.expect('}')?;
        }

        Ok(query)
    }

    /// Parse query parameters: (func: ..., first: 10, ...)
    fn parse_query_params(&mut self, query: &mut Query) -> Result<(), ParseError> {
        self.expect('(')?;
        self.skip_whitespace();

        loop {
            self.skip_whitespace();
            if self.peek() == Some(')') {
                break;
            }
            if self.peek() == Some(',') {
                self.advance();
                self.skip_whitespace();
                continue;
            }

            // Parse parameter
            if self.peek_str("func:") {
                self.advance_by(5);
                self.skip_whitespace();
                query.func = Some(self.parse_function()?);
            } else if self.peek_str("first:") {
                self.advance_by(6);
                self.skip_whitespace();
                query.first = Some(self.parse_int()? as u32);
            } else if self.peek_str("offset:") {
                self.advance_by(7);
                self.skip_whitespace();
                query.offset = Some(self.parse_int()? as u32);
            } else if self.peek_str("after:") {
                self.advance_by(6);
                self.skip_whitespace();
                query.after = Some(self.parse_uid_value()?);
            } else if self.peek_str("orderasc:") {
                self.advance_by(9);
                self.skip_whitespace();
                let attr = self.parse_identifier()?;
                query.order = Some(Order {
                    attr,
                    desc: false,
                    langs: Vec::new(),
                });
            } else if self.peek_str("orderdesc:") {
                self.advance_by(10);
                self.skip_whitespace();
                let attr = self.parse_identifier()?;
                query.order = Some(Order {
                    attr,
                    desc: true,
                    langs: Vec::new(),
                });
            } else if self.peek() != Some(')') {
                return Err(self.error("unexpected token in query parameters"));
            }
        }

        self.expect(')')?;
        Ok(())
    }

    /// Parse directives (@filter, @normalize, @recurse, etc.)
    fn parse_directives(&mut self, query: &mut Query) -> Result<(), ParseError> {
        loop {
            self.skip_whitespace();
            if self.peek() != Some('@') {
                break;
            }

            self.advance(); // skip @
            let directive = self.parse_identifier()?;
            self.skip_whitespace();

            match directive.as_str() {
                "filter" => {
                    self.expect('(')?;
                    query.filter = Some(self.parse_filter()?);
                    self.expect(')')?;
                }
                "normalize" => {
                    query.normalize = true;
                }
                "recurse" => {
                    query.recurse = true;
                    if self.peek() == Some('(') {
                        self.advance();
                        query.recurse_args = self.parse_recurse_args()?;
                        self.expect(')')?;
                    }
                }
                "cascade" => {
                    if self.peek() == Some('(') {
                        self.advance();
                        // Parse specific predicates
                        loop {
                            self.skip_whitespace();
                            if self.peek() == Some(')') {
                                break;
                            }
                            if self.peek() == Some(',') {
                                self.advance();
                                continue;
                            }
                            let pred = self.parse_identifier()?;
                            query.cascade.push(pred);
                        }
                        self.expect(')')?;
                    } else {
                        query.cascade.push("__all__".to_string());
                    }
                }
                "ignorereflex" => {
                    query.ignore_reflex = true;
                }
                "groupby" => {
                    query.is_groupby = true;
                    self.expect('(')?;
                    loop {
                        self.skip_whitespace();
                        if self.peek() == Some(')') {
                            break;
                        }
                        if self.peek() == Some(',') {
                            self.advance();
                            continue;
                        }
                        let attr = self.parse_identifier()?;
                        query.groupby_attrs.push(GroupByAttr {
                            attr,
                            alias: None,
                            langs: Vec::new(),
                        });
                    }
                    self.expect(')')?;
                }
                "facets" => {
                    query.facets = Some(self.parse_facets_directive()?);
                }
                _ => {
                    return Err(self.error(format!("unknown directive: @{directive}")));
                }
            }
        }

        Ok(())
    }

    /// Parse @recurse arguments
    fn parse_recurse_args(&mut self) -> Result<RecurseArgs, ParseError> {
        let mut args = RecurseArgs::default();

        loop {
            self.skip_whitespace();
            if self.peek() == Some(')') {
                break;
            }
            if self.peek() == Some(',') {
                self.advance();
                continue;
            }

            if self.peek_str("depth:") {
                self.advance_by(6);
                self.skip_whitespace();
                args.depth = Some(self.parse_int()? as u64);
            } else if self.peek_str("loop:") {
                self.advance_by(5);
                self.skip_whitespace();
                args.allow_loop = self.parse_bool_value()?;
            } else {
                return Err(self.error("expected 'depth:' or 'loop:' in @recurse"));
            }
        }

        Ok(args)
    }

    /// Parse @facets directive
    fn parse_facets_directive(&mut self) -> Result<FacetParams, ParseError> {
        let mut params = FacetParams::default();

        if self.peek() != Some('(') {
            params.all_keys = true;
            return Ok(params);
        }

        self.advance();
        self.skip_whitespace();

        loop {
            if self.peek() == Some(')') {
                break;
            }
            if self.peek() == Some(',') {
                self.advance();
                self.skip_whitespace();
                continue;
            }

            // Check for alias: "alias as key"
            let first = self.parse_identifier()?;
            self.skip_whitespace();

            if self.peek_str("as ") {
                self.advance_by(2);
                self.skip_whitespace();
                let key = self.parse_identifier()?;
                params.params.push(FacetParam {
                    key,
                    alias: Some(first),
                });
            } else {
                params.params.push(FacetParam {
                    key: first,
                    alias: None,
                });
            }
            self.skip_whitespace();
        }

        self.expect(')')?;
        Ok(params)
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

    /// Parse a function: funcname(args) or count(predicate)
    fn parse_function(&mut self) -> Result<Function, ParseError> {
        let name_str = self.parse_identifier()?;

        // Handle count(predicate) specially
        if name_str == "count" {
            self.skip_whitespace();
            self.expect('(')?;
            self.skip_whitespace();
            let pred = self.parse_identifier()?;
            self.skip_whitespace();
            self.expect(')')?;

            return Ok(Function {
                name: FuncName::Count,
                attr: Some(pred),
                lang: None,
                args: Vec::new(),
                uids: Vec::new(),
                needs_var: Vec::new(),
                is_count: true,
                is_value_var: false,
                is_len_var: false,
            });
        }

        // Handle val(variable) and len(variable)
        if name_str == "val" || name_str == "len" {
            self.skip_whitespace();
            self.expect('(')?;
            self.skip_whitespace();
            let var = self.parse_identifier()?;
            self.skip_whitespace();
            self.expect(')')?;

            let name = FuncName::from_str(&name_str)
                .ok_or_else(|| self.error(format!("unknown function: {name_str}")))?;

            return Ok(Function {
                name,
                attr: None,
                lang: None,
                args: Vec::new(),
                uids: Vec::new(),
                needs_var: vec![VarContext {
                    name: var,
                    typ: VarType::Value,
                }],
                is_count: false,
                is_value_var: name_str == "val",
                is_len_var: name_str == "len",
            });
        }

        let name = FuncName::from_str(&name_str)
            .ok_or_else(|| self.error(format!("unknown function: {name_str}")))?;

        self.skip_whitespace();
        self.expect('(')?;

        let mut args = Vec::new();
        let mut uids = Vec::new();
        let mut attr = None;
        let mut needs_var = Vec::new();

        // For uid function, parse uid list
        if name == FuncName::Uid {
            loop {
                self.skip_whitespace();
                if self.peek() == Some(')') {
                    break;
                }
                if !uids.is_empty() {
                    self.expect(',')?;
                    self.skip_whitespace();
                }

                // Check for variable
                if self.peek() == Some('$') {
                    self.advance();
                    let var_name = self.parse_identifier()?;
                    needs_var.push(VarContext {
                        name: var_name,
                        typ: VarType::Uid,
                    });
                } else if self.peek_str("0x") || matches!(self.peek(), Some('0'..='9')) {
                    uids.push(self.parse_uid_value()?);
                } else {
                    // Could be a variable name without $
                    let name = self.parse_identifier()?;
                    needs_var.push(VarContext {
                        name,
                        typ: VarType::Uid,
                    });
                }
            }
        } else {
            // Regular function: first arg is typically the predicate
            self.skip_whitespace();
            if self.peek() != Some(')') {
                let first_arg = self.parse_value()?;

                // Check if it's a predicate (identifier)
                if let Value::String(s) = &first_arg {
                    if !s.contains(' ') && !s.starts_with('"') {
                        attr = Some(s.clone());
                    } else {
                        args.push(first_arg);
                    }
                } else {
                    args.push(first_arg);
                }

                // Parse remaining arguments
                loop {
                    self.skip_whitespace();
                    if self.peek() == Some(')') {
                        break;
                    }
                    self.expect(',')?;
                    self.skip_whitespace();
                    args.push(self.parse_value()?);
                }
            }
        }

        self.expect(')')?;

        Ok(Function {
            name,
            attr,
            lang: None,
            args,
            uids,
            needs_var,
            is_count: false,
            is_value_var: false,
            is_len_var: false,
        })
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
            Some('[') => {
                // Array/List or Vector
                self.advance();
                let mut items = Vec::new();
                let mut is_numeric = true;

                loop {
                    self.skip_whitespace();
                    if self.peek() == Some(']') {
                        break;
                    }
                    if !items.is_empty() {
                        self.expect(',')?;
                        self.skip_whitespace();
                    }
                    let val = self.parse_value()?;
                    if !matches!(val, Value::Float(_) | Value::Int(_)) {
                        is_numeric = false;
                    }
                    items.push(val);
                }
                self.expect(']')?;

                // If all numeric, return as Vector
                if is_numeric && !items.is_empty() {
                    let vec: Vec<f64> = items
                        .iter()
                        .filter_map(|v| v.as_float())
                        .collect();
                    if vec.len() == items.len() {
                        return Ok(Value::Vector(vec));
                    }
                }

                Ok(Value::List(items))
            }
            Some('/') => {
                // Regex
                self.advance();
                let start = self.pos;
                while self.peek() != Some('/') && !self.is_eof() {
                    if self.peek() == Some('\\') {
                        self.advance();
                    }
                    self.advance();
                }
                let pattern = self.input[start..self.pos].to_string();
                self.expect('/')?;
                Ok(Value::Regex(pattern))
            }
            Some('0'..='9') | Some('-') => {
                let start = self.pos;
                if self.peek() == Some('-') {
                    self.advance();
                }

                // Check for hex
                if self.peek_str("0x") || self.peek_str("0X") {
                    self.advance_by(2);
                    while matches!(self.peek(), Some('0'..='9' | 'a'..='f' | 'A'..='F')) {
                        self.advance();
                    }
                    let hex = &self.input[start + 2..self.pos];
                    let uid = u64::from_str_radix(hex, 16)
                        .map_err(|_| self.error("invalid hex uid"))?;
                    return Ok(Value::Uid(uid));
                }

                // Regular number
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
            Some(c) if c.is_alphabetic() || c == '_' => {
                // Check for val(var)
                if self.peek_str("val(") {
                    self.advance_by(4);
                    let var = self.parse_identifier()?;
                    self.expect(')')?;
                    return Ok(Value::ValVar(var));
                }

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
            } else if self.peek_str("cond") || self.peek() == Some('@') {
                // @if condition
                if self.peek() == Some('@') {
                    self.advance();
                }
                if self.peek_str("if") {
                    self.advance_by(2);
                    self.skip_whitespace();
                    self.expect('(')?;
                    let start = self.pos;
                    let mut depth = 1;
                    while depth > 0 && !self.is_eof() {
                        match self.peek() {
                            Some('(') => depth += 1,
                            Some(')') => depth -= 1,
                            _ => {}
                        }
                        if depth > 0 {
                            self.advance();
                        }
                    }
                    mutation.cond = Some(self.input[start..self.pos].to_string());
                    self.expect(')')?;
                }
            } else {
                return Err(self.error("expected 'set', 'delete', or '@if'"));
            }
        }

        self.expect('}')?;
        Ok(mutation)
    }

    /// Parse RDF triples
    fn parse_triples(&mut self) -> Result<Vec<Triple>, ParseError> {
        let mut triples = Vec::new();

        loop {
            self.skip_whitespace();
            if self.peek() == Some('}') {
                break;
            }

            // Parse subject
            let subject = self.parse_subject()?;
            self.skip_whitespace();

            // Parse predicate
            let predicate = self.parse_predicate()?;
            self.skip_whitespace();

            // Parse object
            let object = self.parse_object()?;
            self.skip_whitespace();

            // Optional language tag or facets
            let mut lang = None;
            let mut facets = Vec::new();

            if self.peek() == Some('@') && !self.peek_str("@facets") {
                self.advance();
                lang = Some(self.parse_identifier()?);
            }

            if self.peek_str("@facets") || self.peek() == Some('(') {
                // Parse facets
                if self.peek_str("@facets") {
                    self.advance_by(7);
                }
                if self.peek() == Some('(') {
                    self.advance();
                    loop {
                        self.skip_whitespace();
                        if self.peek() == Some(')') {
                            break;
                        }
                        if !facets.is_empty() {
                            self.expect(',')?;
                            self.skip_whitespace();
                        }
                        let key = self.parse_identifier()?;
                        self.skip_whitespace();
                        self.expect('=')?;
                        self.skip_whitespace();
                        let val = self.parse_value()?;
                        facets.push((key, val));
                    }
                    self.expect(')')?;
                }
            }

            self.skip_whitespace();
            if self.peek() == Some('.') {
                self.advance();
            }

            triples.push(Triple {
                subject,
                predicate,
                object,
                facets,
                lang,
            });
        }

        Ok(triples)
    }

    fn parse_subject(&mut self) -> Result<Subject, ParseError> {
        self.skip_whitespace();

        if self.peek() == Some('<') {
            // UID: <0x1>
            self.advance();
            if self.peek_str("0x") {
                self.advance_by(2);
                let start = self.pos;
                while matches!(self.peek(), Some('0'..='9' | 'a'..='f' | 'A'..='F')) {
                    self.advance();
                }
                let hex = &self.input[start..self.pos];
                let uid = u64::from_str_radix(hex, 16)
                    .map_err(|_| self.error("invalid hex uid"))?;
                self.expect('>')?;
                return Ok(Subject::Uid(
                    dgraph_common::Uid::new(uid).map_err(|e| self.error(e.to_string()))?,
                ));
            }
            return Err(self.error("expected uid in subject"));
        }

        if self.peek() == Some('_') && self.peek_str("_:") {
            // Blank node: _:name
            self.advance_by(2);
            let name = self.parse_identifier()?;
            return Ok(Subject::Blank(name));
        }

        if self.peek() == Some('u') && self.peek_str("uid(") {
            // uid(variable)
            self.advance_by(4);
            let var = self.parse_identifier()?;
            self.expect(')')?;
            return Ok(Subject::Var(var));
        }

        Err(self.error("expected subject (uid or blank node)"))
    }

    fn parse_predicate(&mut self) -> Result<String, ParseError> {
        self.skip_whitespace();

        if self.peek() == Some('<') {
            self.advance();
            let start = self.pos;
            while self.peek() != Some('>') && !self.is_eof() {
                self.advance();
            }
            let pred = self.input[start..self.pos].to_string();
            self.expect('>')?;
            return Ok(pred);
        }

        self.parse_identifier()
    }

    fn parse_object(&mut self) -> Result<Object, ParseError> {
        self.skip_whitespace();

        if self.peek() == Some('<') {
            // UID reference
            self.advance();
            if self.peek_str("0x") {
                self.advance_by(2);
                let start = self.pos;
                while matches!(self.peek(), Some('0'..='9' | 'a'..='f' | 'A'..='F')) {
                    self.advance();
                }
                let hex = &self.input[start..self.pos];
                let uid = u64::from_str_radix(hex, 16)
                    .map_err(|_| self.error("invalid hex uid"))?;
                self.expect('>')?;
                return Ok(Object::Uid(
                    dgraph_common::Uid::new(uid).map_err(|e| self.error(e.to_string()))?,
                ));
            }
            return Err(self.error("expected uid in object"));
        }

        if self.peek() == Some('_') && self.peek_str("_:") {
            self.advance_by(2);
            let name = self.parse_identifier()?;
            return Ok(Object::Blank(name));
        }

        if self.peek() == Some('u') && self.peek_str("uid(") {
            self.advance_by(4);
            let var = self.parse_identifier()?;
            self.expect(')')?;
            return Ok(Object::Var(var));
        }

        // Value
        let val = self.parse_value()?;
        Ok(Object::Value(val))
    }

    // === Helper methods ===

    fn parse_identifier(&mut self) -> Result<String, ParseError> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_' || c == '.' || c == '*') {
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

    fn parse_uid_value(&mut self) -> Result<u64, ParseError> {
        if self.peek_str("0x") || self.peek_str("0X") {
            self.advance_by(2);
            let start = self.pos;
            while matches!(self.peek(), Some('0'..='9' | 'a'..='f' | 'A'..='F')) {
                self.advance();
            }
            let hex = &self.input[start..self.pos];
            u64::from_str_radix(hex, 16).map_err(|_| self.error("invalid hex uid"))
        } else {
            let start = self.pos;
            while matches!(self.peek(), Some('0'..='9')) {
                self.advance();
            }
            self.input[start..self.pos]
                .parse()
                .map_err(|_| self.error("invalid uid"))
        }
    }

    fn parse_bool_value(&mut self) -> Result<bool, ParseError> {
        if self.peek_str("true") {
            self.advance_by(4);
            Ok(true)
        } else if self.peek_str("false") {
            self.advance_by(5);
            Ok(false)
        } else {
            Err(self.error("expected 'true' or 'false'"))
        }
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

    /// Check if the identifier is an aggregation function used in child queries
    fn is_aggregation_function(name: &str) -> bool {
        matches!(name, "count" | "sum" | "avg" | "min" | "max")
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn error(&self, msg: impl Into<String>) -> ParseError {
        ParseError::new(self.pos, msg)
    }
}
