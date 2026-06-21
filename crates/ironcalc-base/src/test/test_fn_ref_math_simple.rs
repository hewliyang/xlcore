#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_address() {
    let mut model = new_empty_model();

    model._set("A1", "=ADDRESS(2,3)");
    model._set("A2", "=ADDRESS(2,3,2)");
    model._set("A3", "=ADDRESS(2,3,3)");
    model._set("A4", "=ADDRESS(2,3,4)");
    model._set("A5", "=ADDRESS(2,3,1,FALSE)");
    model._set("A6", r#"=ADDRESS(2,3,1,TRUE,"Sheet1")"#);
    model._set("A7", "=ADDRESS(0,3)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"$C$2");
    assert_eq!(model._get_text("A2"), *"C$2");
    assert_eq!(model._get_text("A3"), *"$C2");
    assert_eq!(model._get_text("A4"), *"C2");
    assert_eq!(model._get_text("A5"), *"R2C3");
    assert_eq!(model._get_text("A6"), *"Sheet1!$C$2");
    assert_eq!(model._get_text("A7"), *"#VALUE!");
}

#[test]
fn fn_areas() {
    let mut model = new_empty_model();

    model._set("A1", "=AREAS(B1:C10)");
    model._set("A2", "=AREAS(3)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("A2"), *"#VALUE!");
}

#[test]
fn fn_multinomial() {
    let mut model = new_empty_model();

    model._set("A1", "=MULTINOMIAL(2,3,4)");
    model._set("A2", "=MULTINOMIAL(-1)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"1260");
    assert_eq!(model._get_text("A2"), *"#NUM!");
}

#[test]
fn fn_seriessum() {
    let mut model = new_empty_model();

    model._set("B1", "1");
    model._set("B2", "1");
    model._set("B3", "1");
    model._set("A1", "=SERIESSUM(2,0,1,B1:B3)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"7");
}

#[test]
fn fn_fvschedule() {
    let mut model = new_empty_model();

    model._set("B1", "0.1");
    model._set("B2", "0.2");
    model._set("A1", "=FVSCHEDULE(100,B1:B2)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"132");
}

#[test]
fn fn_permutationa() {
    let mut model = new_empty_model();

    model._set("A1", "=PERMUTATIONA(3,2)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"9");
}

#[test]
fn fn_hyperlink() {
    let mut model = new_empty_model();

    model._set("A1", r#"=HYPERLINK("http://x","label")"#);
    model._set("A2", r#"=HYPERLINK("http://x")"#);

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"label");
    assert_eq!(model._get_text("A2"), *"http://x");
}
