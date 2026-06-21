#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_textsplit_columns() {
    let mut model = new_empty_model();
    model._set("A1", "=TEXTSPLIT(\"a,b,c\",\",\")");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"a");
    assert_eq!(model._get_text("B1"), *"b");
    assert_eq!(model._get_text("C1"), *"c");
}

#[test]
fn fn_textsplit_rows_and_cols() {
    let mut model = new_empty_model();
    model._set("A1", "=TEXTSPLIT(\"a,b;c,d\",\",\",\";\")");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"a");
    assert_eq!(model._get_text("B1"), *"b");
    assert_eq!(model._get_text("A2"), *"c");
    assert_eq!(model._get_text("B2"), *"d");
}

#[test]
fn fn_textsplit_ignore_empty() {
    let mut model = new_empty_model();
    model._set("A1", "=TEXTSPLIT(\"a,,b\",\",\",,TRUE)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"a");
    assert_eq!(model._get_text("B1"), *"b");
}

#[test]
fn fn_textsplit_keep_empty() {
    let mut model = new_empty_model();
    model._set("A1", "=TEXTSPLIT(\"a,,b\",\",\")");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"a");
    assert_eq!(model._get_text("B1"), *"");
    assert_eq!(model._get_text("C1"), *"b");
}

#[test]
fn fn_textsplit_ragged_padding() {
    let mut model = new_empty_model();
    model._set("A1", "=TEXTSPLIT(\"a,b,c;d\",\",\",\";\")");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"a");
    assert_eq!(model._get_text("B1"), *"b");
    assert_eq!(model._get_text("C1"), *"c");
    assert_eq!(model._get_text("A2"), *"d");
    assert_eq!(model._get_text("B2"), *"#N/A");
    assert_eq!(model._get_text("C2"), *"#N/A");
}

#[test]
fn fn_textsplit_case_insensitive() {
    let mut model = new_empty_model();
    model._set("A1", "=TEXTSPLIT(\"aXbxc\",\"x\",,,1)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"a");
    assert_eq!(model._get_text("B1"), *"b");
    assert_eq!(model._get_text("C1"), *"c");
}

#[test]
fn fn_textsplit_args_number() {
    let mut model = new_empty_model();
    model._set("A1", "=TEXTSPLIT(\"a,b\")");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
}
