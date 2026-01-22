//! Tests for dgraph-query

use dgraph_query::{FuncName, Operation, ParseError, Parser, Query, Value, Filter, Function, Order};

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
        assert_eq!(func.uids.len(), 3);
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
        assert_eq!(func.attr, Some("name".to_string()));
        assert_eq!(func.args.len(), 1);
        assert_eq!(func.args[0], Value::String("Alice".to_string()));
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
        assert_eq!(func.attr, Some("email".to_string()));
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
        if let Some(Filter::Func(f)) = &q.filter {
            assert_eq!(f.name, FuncName::Ge);
        }
    }
}

#[test]
fn parse_comment() {
    let query = r#"
    # This is a comment
    {
        # Another comment
        me(func: uid(0x1)) {
            name
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
            if func.name == FuncName::Uid {
                // For uid function, check uids vector
                if let Value::Uid(expected_uid) = expected_value {
                    assert_eq!(func.uids[0], expected_uid, "failed for input: {input}");
                }
            } else {
                // For other functions, check args
                let last_arg = func.args.last().unwrap();
                assert_eq!(*last_arg, expected_value, "failed for input: {input}");
            }
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
// Directive Tests
// ============================================================================

#[test]
fn parse_normalize_directive() {
    let query = r#"{ q(func: uid(0x1)) @normalize { name } }"#;

    let mut parser = Parser::new(query);
    let result = parser.parse().unwrap();

    if let Operation::Query(q) = result {
        assert!(q.normalize);
    }
}

#[test]
fn parse_recurse_directive() {
    let query = r#"{ q(func: uid(0x1)) @recurse(depth: 5) { name friends } }"#;

    let mut parser = Parser::new(query);
    let result = parser.parse().unwrap();

    if let Operation::Query(q) = result {
        assert!(q.recurse);
        assert_eq!(q.recurse_args.depth, Some(5));
    }
}

#[test]
fn parse_cascade_directive() {
    let query = r#"{ q(func: uid(0x1)) @cascade { name } }"#;

    let mut parser = Parser::new(query);
    let result = parser.parse().unwrap();

    if let Operation::Query(q) = result {
        assert!(!q.cascade.is_empty());
        assert!(q.cascade.contains(&"__all__".to_string()));
    }
}

#[test]
fn parse_variable_definition() {
    let query = r#"{ x as q(func: uid(0x1)) { name } }"#;

    let mut parser = Parser::new(query);
    let result = parser.parse().unwrap();

    if let Operation::Query(q) = result {
        assert_eq!(q.var, Some("x".to_string()));
        assert_eq!(q.alias, Some("q".to_string()));
    }
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

#[test]
fn value_as_float() {
    let v = Value::Float(3.14);
    assert_eq!(v.as_float(), Some(3.14));

    let v = Value::Int(42);
    assert_eq!(v.as_float(), Some(42.0)); // Int can be converted to float
}

#[test]
fn value_as_bool() {
    let v = Value::Bool(true);
    assert_eq!(v.as_bool(), Some(true));

    let v = Value::Int(1);
    assert_eq!(v.as_bool(), None);
}

#[test]
fn value_is_var() {
    assert!(Value::Var("x".to_string()).is_var());
    assert!(Value::ValVar("y".to_string()).is_var());
    assert!(!Value::Int(42).is_var());
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
    assert_eq!(FuncName::from_str("type"), Some(FuncName::Type));
    assert_eq!(FuncName::from_str("count"), Some(FuncName::Count));
    assert_eq!(FuncName::from_str("val"), Some(FuncName::Val));
    assert_eq!(FuncName::from_str("len"), Some(FuncName::Len));
    assert_eq!(FuncName::from_str("uid_in"), Some(FuncName::UidIn));
    assert_eq!(FuncName::from_str("similar_to"), Some(FuncName::SimilarTo));
    assert_eq!(FuncName::from_str("unknown"), None);
}

#[test]
fn funcname_is_comparison() {
    assert!(FuncName::Eq.is_comparison());
    assert!(FuncName::Ge.is_comparison());
    assert!(FuncName::Le.is_comparison());
    assert!(FuncName::Gt.is_comparison());
    assert!(FuncName::Lt.is_comparison());
    assert!(FuncName::Between.is_comparison());
    assert!(!FuncName::Has.is_comparison());
    assert!(!FuncName::Uid.is_comparison());
}

#[test]
fn funcname_is_term() {
    assert!(FuncName::AllOfTerms.is_term());
    assert!(FuncName::AnyOfTerms.is_term());
    assert!(FuncName::AllOfText.is_term());
    assert!(FuncName::AnyOfText.is_term());
    assert!(!FuncName::Eq.is_term());
}

#[test]
fn funcname_is_geo() {
    assert!(FuncName::Near.is_geo());
    assert!(FuncName::Within.is_geo());
    assert!(FuncName::Contains.is_geo());
    assert!(FuncName::Intersects.is_geo());
    assert!(!FuncName::Eq.is_geo());
}

// ============================================================================
// Complex Query Tests
// ============================================================================

#[test]
fn parse_complex_filter() {
    let query = r#"{ q(func: uid(0x1)) @filter(ge(age, 21) AND lt(age, 65)) { name } }"#;

    let mut parser = Parser::new(query);
    let result = parser.parse().unwrap();

    if let Operation::Query(q) = result {
        assert!(q.filter.is_some());
        if let Some(Filter::And(filters)) = &q.filter {
            assert_eq!(filters.len(), 2);
        }
    }
}

#[test]
fn parse_or_filter() {
    let query = r#"{ q(func: uid(0x1)) @filter(eq(status, "active") OR eq(status, "pending")) { name } }"#;

    let mut parser = Parser::new(query);
    let result = parser.parse().unwrap();

    if let Operation::Query(q) = result {
        assert!(q.filter.is_some());
        if let Some(Filter::Or(filters)) = &q.filter {
            assert_eq!(filters.len(), 2);
        }
    }
}

#[test]
fn parse_not_filter() {
    let query = r#"{ q(func: uid(0x1)) @filter(NOT eq(deleted, true)) { name } }"#;

    let mut parser = Parser::new(query);
    let result = parser.parse().unwrap();

    if let Operation::Query(q) = result {
        assert!(q.filter.is_some());
        matches!(&q.filter, Some(Filter::Not(_)));
    }
}

#[test]
fn parse_count_function() {
    let query = r#"{ q(func: has(name)) { count(friends) } }"#;

    let mut parser = Parser::new(query);
    let result = parser.parse().unwrap();

    if let Operation::Query(q) = result {
        assert_eq!(q.children.len(), 1);
        // The count is parsed as a predicate with attr = "count(friends)"
    }
}

#[test]
fn parse_type_function() {
    let query = r#"{ q(func: type(Person)) { name } }"#;

    let mut parser = Parser::new(query);
    let result = parser.parse().unwrap();

    if let Operation::Query(q) = result {
        let func = q.func.unwrap();
        assert_eq!(func.name, FuncName::Type);
    }
}

// ============================================================================
// Function Constructor Tests
// ============================================================================

#[test]
fn function_constructors() {
    let uid_func = Function::uid(vec![0x1, 0x2, 0x3]);
    assert_eq!(uid_func.name, FuncName::Uid);
    assert_eq!(uid_func.uids, vec![0x1, 0x2, 0x3]);

    let has_func = Function::has("email");
    assert_eq!(has_func.name, FuncName::Has);
    assert_eq!(has_func.attr, Some("email".to_string()));

    let eq_func = Function::eq("name", Value::String("Alice".to_string()));
    assert_eq!(eq_func.name, FuncName::Eq);
    assert_eq!(eq_func.attr, Some("name".to_string()));
}

// ============================================================================
// Filter Constructor Tests
// ============================================================================

#[test]
fn filter_constructors() {
    let f1 = Filter::func(Function::has("name"));
    let f2 = Filter::func(Function::has("email"));

    let and = Filter::and(vec![f1.clone(), f2.clone()]);
    matches!(and, Filter::And(_));

    let or = Filter::or(vec![f1.clone(), f2.clone()]);
    matches!(or, Filter::Or(_));

    let not = Filter::not(f1);
    matches!(not, Filter::Not(_));
}

// ============================================================================
// Order Constructor Tests
// ============================================================================

#[test]
fn order_constructors() {
    let asc = Order::asc("age");
    assert_eq!(asc.attr, "age");
    assert!(!asc.desc);

    let desc = Order::desc("created_at");
    assert_eq!(desc.attr, "created_at");
    assert!(desc.desc);
}
