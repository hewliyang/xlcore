#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_ecma_ceiling_alias() {
    let mut model = new_empty_model();

    model._set("A1", "=ECMA.CEILING(4.3)");
    model._set("A2", "=ECMA.CEILING(-4.3, 1)");
    model._set("B1", "=ISO.CEILING(4.3)");
    model._set("B2", "=ISO.CEILING(-4.3, 1)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("B1"));
    assert_eq!(model._get_text("A2"), model._get_text("B2"));
}

#[test]
fn fn_usdollar_alias() {
    let mut model = new_empty_model();

    model._set("A1", "=USDOLLAR(1234.5, 2)");
    model._set("B1", "=DOLLAR(1234.5, 2)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"$1,234.50");
    assert_eq!(model._get_text("A1"), model._get_text("B1"));
}
