#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn valuetotext_concise() {
    let mut model = new_empty_model();
    model._set("A1", "=VALUETOTEXT(12.34)");
    model._set("A2", "=VALUETOTEXT(TRUE)");
    model._set("A3", "=VALUETOTEXT(\"abc\")");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"12.34");
    assert_eq!(model._get_text("A2"), *"TRUE");
    assert_eq!(model._get_text("A3"), *"abc");
}

#[test]
fn valuetotext_strict() {
    let mut model = new_empty_model();
    model._set("A1", "=VALUETOTEXT(\"abc\",1)");
    model._set("A2", "=VALUETOTEXT(12.34,1)");
    model._set("A3", "=VALUETOTEXT(TRUE,1)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"\"abc\"");
    assert_eq!(model._get_text("A2"), *"12.34");
    assert_eq!(model._get_text("A3"), *"TRUE");
}

#[test]
fn valuetotext_bad_format() {
    let mut model = new_empty_model();
    model._set("A1", "=VALUETOTEXT(1,2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#VALUE!");
}

#[test]
fn valuetotext_range_topleft() {
    let mut model = new_empty_model();
    model._set("A1", "hello");
    model._set("A2", "world");
    model._set("B1", "=VALUETOTEXT(A1:A2)");
    model.evaluate();
    assert_eq!(model._get_text("B1"), *"hello");
}

#[test]
fn bahttext_basic() {
    let mut model = new_empty_model();
    model._set("A1", "=BAHTTEXT(1)");
    model._set("A2", "=BAHTTEXT(21)");
    model._set("A3", "=BAHTTEXT(1234.5)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"หนึ่งบาทถ้วน");
    assert_eq!(model._get_text("A2"), *"ยี่สิบเอ็ดบาทถ้วน");
    assert_eq!(model._get_text("A3"), *"หนึ่งพันสองร้อยสามสิบสี่บาทห้าสิบสตางค์");
}

#[test]
fn bahttext_zero_and_negative() {
    let mut model = new_empty_model();
    model._set("A1", "=BAHTTEXT(0)");
    model._set("A2", "=BAHTTEXT(-5)");
    model._set("A3", "=BAHTTEXT(10)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"ศูนย์บาทถ้วน");
    assert_eq!(model._get_text("A2"), *"ลบห้าบาทถ้วน");
    assert_eq!(model._get_text("A3"), *"สิบบาทถ้วน");
}

#[test]
fn bahttext_millions() {
    let mut model = new_empty_model();
    model._set("A1", "=BAHTTEXT(2000000)");
    model._set("A2", "=BAHTTEXT(101)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"สองล้านบาทถ้วน");
    assert_eq!(model._get_text("A2"), *"หนึ่งร้อยเอ็ดบาทถ้วน");
}
