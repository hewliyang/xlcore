#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_lenb_matches_len() {
    let mut model = new_empty_model();
    model._set("A1", "=LENB(\"hello\")");
    model._set("A2", "=LEN(\"hello\")");
    model.evaluate();
    assert_eq!(model._get_text("A1"), "5");
    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_leftb_matches_left() {
    let mut model = new_empty_model();
    model._set("A1", "=LEFTB(\"hello\",2)");
    model._set("A2", "=LEFT(\"hello\",2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), "he");
    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_rightb_matches_right() {
    let mut model = new_empty_model();
    model._set("A1", "=RIGHTB(\"hello\",2)");
    model._set("A2", "=RIGHT(\"hello\",2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), "lo");
    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_midb_matches_mid() {
    let mut model = new_empty_model();
    model._set("A1", "=MIDB(\"hello\",2,3)");
    model._set("A2", "=MID(\"hello\",2,3)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), "ell");
    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_findb_matches_find() {
    let mut model = new_empty_model();
    model._set("A1", "=FINDB(\"l\",\"hello\")");
    model._set("A2", "=FIND(\"l\",\"hello\")");
    model.evaluate();
    assert_eq!(model._get_text("A1"), "3");
    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_searchb_matches_search() {
    let mut model = new_empty_model();
    model._set("A1", "=SEARCHB(\"L\",\"hello\")");
    model._set("A2", "=SEARCH(\"L\",\"hello\")");
    model.evaluate();
    assert_eq!(model._get_text("A1"), "3");
    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_replaceb_matches_replace() {
    let mut model = new_empty_model();
    model._set("A1", "=REPLACEB(\"abcdef\",2,3,\"XY\")");
    model._set("A2", "=REPLACE(\"abcdef\",2,3,\"XY\")");
    model.evaluate();
    assert_eq!(model._get_text("A1"), "aXYef");
    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}
