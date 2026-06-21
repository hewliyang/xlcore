#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_minverse_2x2() {
    let mut model = new_empty_model();
    model._set("G1", "4");
    model._set("H1", "7");
    model._set("G2", "2");
    model._set("H2", "6");
    model._set("A1", "=MINVERSE(G1:H2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"0.6");
    assert_eq!(model._get_text("B1"), *"-0.7");
    assert_eq!(model._get_text("A2"), *"-0.2");
    assert_eq!(model._get_text("B2"), *"0.4");
}

#[test]
fn fn_minverse_singular() {
    let mut model = new_empty_model();
    model._set("G1", "1");
    model._set("H1", "2");
    model._set("G2", "2");
    model._set("H2", "4");
    model._set("A1", "=MINVERSE(G1:H2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#NUM!");
}

#[test]
fn fn_minverse_non_square() {
    let mut model = new_empty_model();
    model._set("G1", "1");
    model._set("H1", "2");
    model._set("I1", "3");
    model._set("G2", "4");
    model._set("H2", "5");
    model._set("I2", "6");
    model._set("A1", "=MINVERSE(G1:I2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#VALUE!");
}
