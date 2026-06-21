#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_randarray_args_number() {
    let mut model = new_empty_model();
    model._set("A1", "=RANDARRAY(1,1,1,1,2,3)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
}

#[test]
fn fn_randarray_default_spills() {
    let mut model = new_empty_model();
    model._set("A1", "=RANDARRAY(2,3)");
    model.evaluate();
    for cell in ["A1", "B1", "C1", "A2", "B2", "C2"] {
        let value: f64 = model._get_text(cell).parse().unwrap();
        assert!((0.0..1.0).contains(&value));
    }
}

#[test]
fn fn_randarray_no_args() {
    let mut model = new_empty_model();
    model._set("A1", "=RANDARRAY()");
    model.evaluate();
    let value: f64 = model._get_text("A1").parse().unwrap();
    assert!((0.0..1.0).contains(&value));
}

#[test]
fn fn_randarray_whole_number_single() {
    let mut model = new_empty_model();
    model._set("Z1", "=RANDARRAY(1,1,5,5,TRUE)");
    model.evaluate();
    assert_eq!(model._get_text("Z1"), *"5");
}

#[test]
fn fn_randarray_whole_number_range() {
    let mut model = new_empty_model();
    model._set("Z1", "=RANDARRAY(4,4,10,20,TRUE)");
    model.evaluate();
    for row in 1..=4 {
        for col in ["Z", "AA", "AB", "AC"] {
            let cell = format!("{col}{row}");
            let value: f64 = model._get_text(&cell).parse().unwrap();
            assert_eq!(value.fract(), 0.0);
            assert!((10.0..=20.0).contains(&value));
        }
    }
}

#[test]
fn fn_randarray_min_greater_than_max() {
    let mut model = new_empty_model();
    model._set("Z1", "=RANDARRAY(1,1,10,5)");
    model.evaluate();
    assert_eq!(model._get_text("Z1"), *"#VALUE!");
}

#[test]
fn fn_randarray_bad_dimensions() {
    let mut model = new_empty_model();
    model._set("Z1", "=RANDARRAY(2,0)");
    model._set("Z3", "=RANDARRAY(-1,2)");
    model.evaluate();
    assert_eq!(model._get_text("Z1"), *"#VALUE!");
    assert_eq!(model._get_text("Z3"), *"#VALUE!");
}

#[test]
fn fn_randarray_non_integer_whole() {
    let mut model = new_empty_model();
    model._set("Z1", "=RANDARRAY(1,1,1.5,5,TRUE)");
    model.evaluate();
    assert_eq!(model._get_text("Z1"), *"#VALUE!");
}
