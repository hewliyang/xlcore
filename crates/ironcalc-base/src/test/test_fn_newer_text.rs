#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_unichar() {
    let mut model = new_empty_model();

    model._set("A1", "=UNICHAR(65)");
    model._set("A2", "=UNICHAR(8364)");
    model._set("A3", "=UNICHAR(0)");
    model._set("A4", "=UNICHAR(1114112)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"A");
    assert_eq!(model._get_text("A2"), *"€");
    assert_eq!(model._get_text("A3"), *"#VALUE!");
    assert_eq!(model._get_text("A4"), *"#VALUE!");
}

#[test]
fn fn_numbervalue() {
    let mut model = new_empty_model();

    model._set("A1", r#"=NUMBERVALUE("2.500,50", ",", ".")"#);
    model._set("A2", r#"=NUMBERVALUE("1,234.56")"#);
    model._set("A3", r#"=NUMBERVALUE("50%")"#);
    model._set("A4", r#"=NUMBERVALUE("")"#);
    model._set("A5", r#"=NUMBERVALUE("1.2.3")"#);

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"2500.5");
    assert_eq!(model._get_text("A2"), *"1234.56");
    assert_eq!(model._get_text("A3"), *"0.5");
    assert_eq!(model._get_text("A4"), *"0");
    assert_eq!(model._get_text("A5"), *"#VALUE!");
}
