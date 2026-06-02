use std::borrow::Cow;
use std::collections::HashMap;

use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;

use crate::error::{FixedAttribute, LoadReport, SchemaErrorKind};
use crate::precompile::fixer::{sax_rewrite, Emit, Fixer, FixerError};

#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub(crate) enum AttrType {
    UnsignedByte,
    UnsignedInt,
    Int,

    Double,
    Boolean,

    Enum(&'static [&'static str]),
}

impl AttrType {
    fn accepts(&self, value: &str) -> bool {
        match self {
            AttrType::UnsignedByte => value.parse::<u8>().is_ok(),
            AttrType::UnsignedInt => value.parse::<u32>().is_ok(),
            AttrType::Int => value.parse::<i32>().is_ok(),
            AttrType::Double => value.parse::<f64>().map(|f| f.is_finite()).unwrap_or(false),
            AttrType::Boolean => matches!(value, "0" | "1" | "true" | "false"),
            AttrType::Enum(variants) => variants.contains(&value),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            AttrType::UnsignedByte => "xsd:unsignedByte",
            AttrType::UnsignedInt => "xsd:unsignedInt",
            AttrType::Int => "xsd:int",
            AttrType::Double => "xsd:double",
            AttrType::Boolean => "xsd:boolean",
            AttrType::Enum(_) => "enum",
        }
    }
}

fn lookup(tag: &[u8], attr: &[u8]) -> Option<AttrType> {
    const TABLE: &[(&[u8], &[u8], AttrType)] = &[
        (b"alignment", b"textRotation", AttrType::UnsignedByte),
        (b"alignment", b"indent", AttrType::UnsignedInt),
        (b"alignment", b"wrapText", AttrType::Boolean),
        (b"alignment", b"shrinkToFit", AttrType::Boolean),
        (b"alignment", b"justifyLastLine", AttrType::Boolean),
    ];
    TABLE
        .iter()
        .find(|(t, a, _)| *t == tag && *a == attr)
        .map(|(_, _, ty)| *ty)
}

pub(crate) struct AttributeTypeSanitizer;

impl Fixer for AttributeTypeSanitizer {
    fn name(&self) -> &'static str {
        "attribute_type_sanitize"
    }

    fn applies_to(&self, part: &str) -> bool {
        // The current lookup table only knows <alignment> attrs, which live in
        // styles.xml. Scope tightly to avoid SAX-scanning every xml part.
        part == "xl/styles.xml"
    }

    fn rewrite(
        &self,
        xml: &[u8],
        part: &str,
        report: &mut LoadReport,
    ) -> Result<Option<Vec<u8>>, FixerError> {
        let mut bucket: HashMap<(Vec<u8>, Vec<u8>, String), usize> = HashMap::new();

        let out = sax_rewrite(xml, part, report, |_ctx, ev| match ev {
            Event::Start(e) => match scrub(&e, &mut bucket) {
                Some(new) => Emit::Replace {
                    event: Event::Start(new),
                    changed: true,
                },
                None => Emit::Keep(Event::Start(e)),
            },
            Event::Empty(e) => match scrub(&e, &mut bucket) {
                Some(new) => Emit::Replace {
                    event: Event::Empty(new),
                    changed: true,
                },
                None => Emit::Keep(Event::Empty(e)),
            },
            other => Emit::Keep(other),
        })?;

        for ((tag, attr, value), count) in bucket {
            report.fixes.push(FixedAttribute {
                part: part.to_string(),
                ty: lookup(&tag, &attr).map(|t| t.name().to_string()),
                field: Some(String::from_utf8_lossy(&attr).into_owned()),
                value: Some(value),
                occurrences: count,
                kind: SchemaErrorKind::InvalidFieldValue,
            });
        }
        Ok(out)
    }
}

fn scrub(
    e: &BytesStart<'_>,
    bucket: &mut HashMap<(Vec<u8>, Vec<u8>, String), usize>,
) -> Option<BytesStart<'static>> {
    let name = e.name();
    let tag_local = local_name(name.as_ref());

    let mut to_drop_any = false;
    for attr in e.attributes().flatten() {
        let key = attr.key.as_ref();
        let attr_local = local_name(key);
        if let Some(ty) = lookup(tag_local, attr_local) {
            if !ty.accepts(std::str::from_utf8(&attr.value).unwrap_or("")) {
                to_drop_any = true;
                break;
            }
        }
    }
    if !to_drop_any {
        return None;
    }

    let name = std::str::from_utf8(e.name().as_ref()).ok()?.to_string();
    let mut out = BytesStart::new(name);
    for attr in e.attributes().flatten() {
        let key = attr.key.as_ref();
        let attr_local = local_name(key);
        if let Some(ty) = lookup(tag_local, attr_local) {
            let v = std::str::from_utf8(&attr.value).unwrap_or("");
            if !ty.accepts(v) {
                *bucket
                    .entry((tag_local.to_vec(), attr_local.to_vec(), v.to_string()))
                    .or_insert(0) += 1;
                continue;
            }
        }

        out.push_attribute(Attribute {
            key: QName(key),
            value: Cow::Owned(attr.value.into_owned()),
        });
    }
    Some(out)
}

fn local_name(qname: &[u8]) -> &[u8] {
    match qname.iter().position(|&b| b == b':') {
        Some(i) => &qname[i + 1..],
        None => qname,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(xml: &str) -> (Option<String>, LoadReport) {
        let mut report = LoadReport::default();
        let out = AttributeTypeSanitizer
            .rewrite(xml.as_bytes(), "xl/styles.xml", &mut report)
            .expect("ok");
        (out.map(|b| String::from_utf8(b).unwrap()), report)
    }

    #[test]
    fn drops_text_rotation_nan() {
        let (out, rep) =
            run(r#"<xf><alignment horizontal="center" textRotation="NaN" wrapText="1"/></xf>"#);
        let s = out.expect("rewritten");
        assert!(!s.contains("textRotation"), "{s}");
        assert!(s.contains(r#"horizontal="center""#), "{s}");
        assert!(s.contains(r#"wrapText="1""#), "{s}");
        assert_eq!(rep.fixes.len(), 1);
        let fix = &rep.fixes[0];
        assert_eq!(fix.field.as_deref(), Some("textRotation"));
        assert_eq!(fix.value.as_deref(), Some("NaN"));
        assert_eq!(fix.ty.as_deref(), Some("xsd:unsignedByte"));
        assert_eq!(fix.occurrences, 1);
    }

    #[test]
    fn leaves_valid_values_alone() {
        let (out, _) = run(r#"<alignment textRotation="90" wrapText="0"/>"#);
        assert!(out.is_none(), "valid values must be untouched");
    }

    #[test]
    fn preserves_valid_sibling_with_same_attribute_name() {
        let (out, _) = run(r#"<alignment textRotation="0" wrapText="1"/>"#);
        assert!(out.is_none());
    }

    #[test]
    fn coalesces_repeats_into_single_fix() {
        let (out, rep) = run(
            r#"<r><alignment textRotation="NaN"/><alignment textRotation="NaN"/><alignment textRotation="NaN"/></r>"#,
        );
        assert!(out.is_some());
        assert_eq!(rep.fixes.len(), 1);
        assert_eq!(rep.fixes[0].occurrences, 3);
    }

    #[test]
    fn distinguishes_different_bad_values() {
        let (out, rep) =
            run(r#"<r><alignment textRotation="NaN"/><alignment textRotation="999"/></r>"#);
        assert!(out.is_some());
        let mut values: Vec<&str> = rep
            .fixes
            .iter()
            .filter_map(|f| f.value.as_deref())
            .collect();
        values.sort();
        assert_eq!(values, vec!["999", "NaN"]);
    }

    #[test]
    fn untyped_attribute_passes_through_regardless_of_value() {
        let (out, _) = run(r#"<alignment bogus="!!!"/>"#);
        assert!(out.is_none());
    }

    #[test]
    fn untyped_element_passes_through() {
        let (out, _) = run(r#"<someThing textRotation="NaN"/>"#);
        assert!(out.is_none());
    }

    #[test]
    fn empty_and_self_closing_forms_both_handled() {
        let (out, _) = run(r#"<alignment textRotation="NaN"/>"#);
        let s = out.expect("rewritten");
        assert!(!s.contains("textRotation"), "{s}");

        assert!(s.contains("<alignment") && s.contains("/>"), "{s}");
    }
}
