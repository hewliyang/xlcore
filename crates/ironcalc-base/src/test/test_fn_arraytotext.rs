#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_arraytotext_args_number() {
    let mut model = new_empty_model();

    model._set("A1", "=ARRAYTOTEXT()");
    model._set("A2", "=ARRAYTOTEXT(1, 0, 0)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"#ERROR!");
    assert_eq!(model._get_text("A2"), *"#ERROR!");
}

#[test]
fn fn_arraytotext_scalar() {
    let mut model = new_empty_model();

    model._set("A1", "=ARRAYTOTEXT(123)");
    model._set("A2", r#"=ARRAYTOTEXT("hello")"#);
    model._set("A3", "=ARRAYTOTEXT(TRUE)");
    model._set("A4", r#"=ARRAYTOTEXT("hello", 1)"#);
    model._set("A5", "=ARRAYTOTEXT(123, 1)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"123");
    assert_eq!(model._get_text("A2"), *"hello");
    assert_eq!(model._get_text("A3"), *"TRUE");
    assert_eq!(model._get_text("A4"), *"{\"hello\"}");
    assert_eq!(model._get_text("A5"), *"{123}");
}

#[test]
fn fn_arraytotext_range_concise() {
    let mut model = new_empty_model();

    model._set("A1", "1");
    model._set("B1", "2");
    model._set("A2", "hello");
    model._set("B2", "world");

    model._set("D1", "=ARRAYTOTEXT(A1:B2)");
    model._set("D2", "=ARRAYTOTEXT(A1:B2, 0)");

    model.evaluate();

    assert_eq!(model._get_text("D1"), *"1, 2, hello, world");
    assert_eq!(model._get_text("D2"), *"1, 2, hello, world");
}

#[test]
fn fn_arraytotext_range_strict() {
    let mut model = new_empty_model();

    model._set("A1", "1");
    model._set("B1", "2");
    model._set("A2", "hello");
    model._set("B2", "world");

    model._set("D1", "=ARRAYTOTEXT(A1:B2, 1)");

    model.evaluate();

    assert_eq!(model._get_text("D1"), *"{1,2;\"hello\",\"world\"}");
}

#[test]
fn fn_arraytotext_errors_and_bools() {
    let mut model = new_empty_model();

    model._set("A1", "=1/0");
    model._set("B1", "=TRUE");

    model._set("D1", "=ARRAYTOTEXT(A1:B1, 1)");
    model._set("D2", "=ARRAYTOTEXT(A1:B1)");

    model.evaluate();

    assert_eq!(model._get_text("D1"), *"{#DIV/0!,TRUE}");
    assert_eq!(model._get_text("D2"), *"#DIV/0!, TRUE");
}

#[test]
fn fn_arraytotext_invalid_format() {
    let mut model = new_empty_model();

    model._set("A1", "=ARRAYTOTEXT(1, 2)");
    model._set("A2", "=ARRAYTOTEXT(1, -1)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"#VALUE!");
    assert_eq!(model._get_text("A2"), *"#VALUE!");
}
