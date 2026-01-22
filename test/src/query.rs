//! Tests for dgraph-query

use dgraph_query::{FuncName, Operation, ParseError, Parser, Query, Value};

// ============================================================================
// Parser Tests
// ============================================================================

#[test]
fn parse_empty_query() {
    let mut parser = Parser::new("{}");
    let result = parser.parse();
    assert!(result.is_ok());
}

#[test]
fn parse_simple_query() {
    let query = r#"
    {
        me(func: uid(0x1)) {
            name
        }
    }
    "#;

    let mut parser = Parser::new(query);
    let result = parser.parse().unwrap();

    match result {
        Operation::Query(q) => {
            assert_eq!(q.alias, Some("me".to_string()));
            assert!(q.func.is_some());
            assert_eq!(q.children.len(), 1);
            assert_eq!(q.children[0].attr, Some("name".to_string()));
        }
        _ => panic!("expected query"),
    }
}

#[test]
fn parse_uid_function() {
    let query = r#"{ q(func: uid(0x1, 0x2, 0x3)) { name } }"#;

    let mut parser = Parser::new(query);
    let result = parser.parse().unwrap();

    if let Operation::Query(q) = result {
        let func = q.func.unwrap();
        assert_eq!(func.name, FuncName::Uid);
        assert_eq!(func.args.len(), 3);
    }
}

#[test]
fn parse_eq_function() {
    let query = r#"{ q(func: eq(name, "Alice")) { age } }"#;

    let mut parser = Parser::new(query);
    let result = parser.parse().unwrap();

    if let Operation::Query(q) = result {
        let func = q.func.unwrap();
        assert_eq!(func.name, FuncName::Eq);
        assert_eq!(func.args.len(), 2);
        assert_eq!(func.args[0], Value::String("name".to_string()));
        assert_eq!(func.args[1], Value::String("Alice".to_string()));
    }
}

#[test]
fn parse_has_function() {
    let query = r#"{ q(func: has(email)) { name } }"#;

    let mut parser = Parser::new(query);
    let result = parser.parse().unwrap();

    if let Operation::Query(q) = result {
        let func = q.func.unwrap();
        assert_eq!(func.name, FuncName::Has);
    }
}

#[test]
fn parse_nested_query() {
    let query = r#"
    {
        me(func: uid(0x1)) {
            name
            friends {
                name
                age
            }
        }
    }
    "#;

    let mut parser = Parser::new(query);
    let result = parser.parse().unwrap();

    if let Operation::Query(q) = result {
        assert_eq!(q.children.len(), 2); // name, friends
        let friends = &q.children[1];
        assert_eq!(friends.attr, Some("friends".to_string()));
        assert_eq!(friends.children.len(), 2); // name, age
    }
}

#[test]
fn parse_first_offset() {
    let query = r#"{ q(func: has(name), first: 10, offset: 5) { name } }"#;

    let mut parser = Parser::new(query);
    let result = parser.parse().unwrap();

    if let Operation::Query(q) = result {
        assert_eq!(q.first, Some(10));
        assert_eq!(q.offset, Some(5));
    }
}

#[test]
fn parse_orderasc() {
    let query = r#"{ q(func: has(name), orderasc: age) { name } }"#;

    let mut parser = Parser::new(query);
    let result = parser.parse().unwrap();

    if let Operation::Query(q) = result {
        let order = q.order.unwrap();
        assert_eq!(order.attr, "age");
        assert!(!order.desc);
    }
}

#[test]
fn parse_orderdesc() {
    let query = r#"{ q(func: has(name), orderdesc: age) { name } }"#;

    let mut parser = Parser::new(query);
    let result = parser.parse().unwrap();

    if let Operation::Query(q) = result {
        let order = q.order.unwrap();
        assert_eq!(order.attr, "age");
        assert!(order.desc);
    }
}

#[test]
fn parse_filter() {
    let query = r#"{ q(func: uid(0x1)) @filter(ge(age, 21)) { name } }"#;

    let mut parser = Parser::new(query);
    let result = parser.parse().unwrap();

    if let Operation::Query(q) = result {
        assert!(q.filter.is_some());
    }
}

#[test]
fn parse_comment() {
    let query = r#"
    # This is a comment
    {
        # Another comment
        me(func: uid(0x1)) {
            name # inline comment doesn't work in this simple parser
        }
    }
    "#;

    let mut parser = Parser::new(query);
    let result = parser.parse();
    assert!(result.is_ok());
}

#[test]
fn parse_values() {
    // Test different value types
    let test_cases = [
        (r#"{ q(func: eq(x, 42)) {} }"#, Value::Int(42)),
        (r#"{ q(func: eq(x, -10)) {} }"#, Value::Int(-10)),
        (r#"{ q(func: eq(x, 3.14)) {} }"#, Value::Float(3.14)),
        (r#"{ q(func: eq(x, "hello")) {} }"#, Value::String("hello".to_string())),
        (r#"{ q(func: eq(x, true)) {} }"#, Value::Bool(true)),
        (r#"{ q(func: eq(x, false)) {} }"#, Value::Bool(false)),
        (r#"{ q(func: uid(0xDEAD)) {} }"#, Value::Uid(0xDEAD)),
    ];

    for (input, expected_value) in test_cases {
        let mut parser = Parser::new(input);
        let result = parser.parse().unwrap();

        if let Operation::Query(q) = result {
            let func = q.func.unwrap();
            let last_arg = func.args.last().unwrap();
            assert_eq!(*last_arg, expected_value, "failed for input: {input}");
        }
    }
}

#[test]
fn parse_error_unclosed_brace() {
    let query = r#"{ me(func: uid(0x1)) { name }"#; // missing closing }

    let mut parser = Parser::new(query);
    let result = parser.parse();
    assert!(result.is_err());
}

#[test]
fn parse_error_unknown_function() {
    let query = r#"{ q(func: unknown(x)) { } }"#;

    let mut parser = Parser::new(query);
    let result = parser.parse();
    assert!(result.is_err());
}

// ============================================================================
// Value Tests
// ============================================================================

#[test]
fn value_as_str() {
    let v = Value::String("hello".to_string());
    assert_eq!(v.as_str(), Some("hello"));

    let v = Value::Int(42);
    assert_eq!(v.as_str(), None);
}

#[test]
fn value_as_int() {
    let v = Value::Int(42);
    assert_eq!(v.as_int(), Some(42));

    let v = Value::String("42".to_string());
    assert_eq!(v.as_int(), None);
}

#[test]
fn value_as_uid() {
    let v = Value::Uid(0x123);
    assert_eq!(v.as_uid(), Some(0x123));

    let v = Value::Int(0x123);
    assert_eq!(v.as_uid(), None);
}

// ============================================================================
// FuncName Tests
// ============================================================================

#[test]
fn funcname_from_str() {
    assert_eq!(FuncName::from_str("uid"), Some(FuncName::Uid));
    assert_eq!(FuncName::from_str("eq"), Some(FuncName::Eq));
    assert_eq!(FuncName::from_str("has"), Some(FuncName::Has));
    assert_eq!(FuncName::from_str("ge"), Some(FuncName::Ge));
    assert_eq!(FuncName::from_str("le"), Some(FuncName::Le));
    assert_eq!(FuncName::from_str("gt"), Some(FuncName::Gt));
    assert_eq!(FuncName::from_str("lt"), Some(FuncName::Lt));
    assert_eq!(FuncName::from_str("allofterms"), Some(FuncName::AllOfTerms));
    assert_eq!(FuncName::from_str("anyofterms"), Some(FuncName::AnyOfTerms));
    assert_eq!(FuncName::from_str("unknown"), None);
}
