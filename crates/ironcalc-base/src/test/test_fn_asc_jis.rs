#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_asc_args_number() {
    let mut model = new_empty_model();

    model._set("A1", "=ASC()");
    model._set("A2", r#"=ASC("a", "b")"#);
    model._set("A3", "=JIS()");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"#ERROR!");
    assert_eq!(model._get_text("A2"), *"#ERROR!");
    assert_eq!(model._get_text("A3"), *"#ERROR!");
}

#[test]
fn fn_asc_identity() {
    let mut model = new_empty_model();

    model._set("A1", r#"=ASC("ABC")"#);
    model._set("A2", r#"=ASC("Hello 123!")"#);

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"ABC");
    assert_eq!(model._get_text("A2"), *"Hello 123!");
}

#[test]
fn fn_asc_fullwidth() {
    let mut model = new_empty_model();

    model._set("A1", r#"=ASC("ＡＢＣ")"#);
    model._set("A2", r#"=ASC("１２３")"#);

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"ABC");
    assert_eq!(model._get_text("A2"), *"123");
}

#[test]
fn fn_jis_halfwidth() {
    let mut model = new_empty_model();

    model._set("A1", r#"=JIS("ABC")"#);
    model._set("A2", r#"=JIS("123")"#);

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"ＡＢＣ");
    assert_eq!(model._get_text("A2"), *"１２３");
}

#[test]
fn fn_asc_jis_round_trip() {
    let mut model = new_empty_model();

    model._set("A1", r#"=ASC(JIS("Hello 123!"))"#);
    model._set("A2", r#"=EXACT(ASC(JIS("Hello 123!")), "Hello 123!")"#);

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"Hello 123!");
    assert_eq!(model._get_text("A2"), *"TRUE");
}
