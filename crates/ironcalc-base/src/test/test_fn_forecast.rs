#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_forecast_classic() {
    let mut model = new_empty_model();

    model._set("A1", "6");
    model._set("A2", "7");
    model._set("A3", "9");
    model._set("A4", "15");
    model._set("A5", "21");
    model._set("B1", "20");
    model._set("B2", "28");
    model._set("B3", "31");
    model._set("B4", "38");
    model._set("B5", "40");

    model._set("C1", "=FORECAST(30, A1:A5, B1:B5)");
    model._set("C2", "=FORECAST.LINEAR(30, A1:A5, B1:B5)");

    model.evaluate();

    assert_eq!(model._get_text("C1"), *"10.607253086");
    assert_eq!(model._get_text("C2"), *"10.607253086");
}

#[test]
fn fn_forecast_errors() {
    let mut model = new_empty_model();

    model._set("A1", "1");
    model._set("A2", "2");
    model._set("B1", "5");
    model._set("B2", "5");

    model._set("C1", "=FORECAST(30, A1:A2)");
    model._set("C2", "=FORECAST(30, A1:A2, B1:B2)");
    model._set("C3", "=FORECAST(30, A1:A1, B1:B1)");

    model.evaluate();

    assert_eq!(model._get_text("C1"), *"#ERROR!");
    assert_eq!(model._get_text("C2"), *"#DIV/0!");
    assert_eq!(model._get_text("C3"), *"#DIV/0!");
}
