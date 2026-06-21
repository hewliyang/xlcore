#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_char() {
    let mut model = new_empty_model();

    model._set("A1", "=CHAR(65)");
    model._set("A2", "=CHAR(0)");
    model._set("A3", "=CHAR(256)");
    model._set("A4", "=CHAR(97.9)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"A");
    assert_eq!(model._get_text("A2"), *"#VALUE!");
    assert_eq!(model._get_text("A3"), *"#VALUE!");
    assert_eq!(model._get_text("A4"), *"a");
}

#[test]
fn fn_code() {
    let mut model = new_empty_model();

    model._set("A1", r#"=CODE("A")"#);
    model._set("A2", r#"=CODE("apple")"#);
    model._set("A3", r#"=CODE("")"#);

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"65");
    assert_eq!(model._get_text("A2"), *"97");
    assert_eq!(model._get_text("A3"), *"#VALUE!");
}

#[test]
fn fn_clean() {
    let mut model = new_empty_model();

    model._set("A1", r#"=CLEAN(CHAR(7)&"text"&CHAR(7))"#);

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"text");
}

#[test]
fn fn_proper() {
    let mut model = new_empty_model();

    model._set("A1", r#"=PROPER("this is a TITLE")"#);
    model._set("A2", r#"=PROPER("2-cent's worth")"#);

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"This Is A Title");
    assert_eq!(model._get_text("A2"), *"2-Cent'S Worth");
}

#[test]
fn fn_replace() {
    let mut model = new_empty_model();

    model._set("A1", r#"=REPLACE("abcdef",2,3,"XY")"#);
    model._set("A2", r#"=REPLACE("abcdef",0,3,"XY")"#);
    model._set("A3", r#"=REPLACE("abcdef",2,-1,"XY")"#);

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"aXYef");
    assert_eq!(model._get_text("A2"), *"#VALUE!");
    assert_eq!(model._get_text("A3"), *"#VALUE!");
}

#[test]
fn fn_fixed() {
    let mut model = new_empty_model();

    model._set("A1", "=FIXED(1234.567,1)");
    model._set("A2", "=FIXED(1234.567,1,TRUE)");
    model._set("A3", "=FIXED(1234.567,-1)");
    model._set("A4", "=FIXED(-1234.567,2)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"1,234.6");
    assert_eq!(model._get_text("A2"), *"1234.6");
    assert_eq!(model._get_text("A3"), *"1,230");
    assert_eq!(model._get_text("A4"), *"-1,234.57");
}

#[test]
fn fn_dollar() {
    let mut model = new_empty_model();

    model._set("A1", "=DOLLAR(1234.5,2)");
    model._set("A2", "=DOLLAR(-1234.5)");
    model._set("A3", "=DOLLAR(1234.5)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"$1,234.50");
    assert_eq!(model._get_text("A2"), *"($1,234.50)");
    assert_eq!(model._get_text("A3"), *"$1,234.50");
}
