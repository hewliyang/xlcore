use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};

use crate::error::{FixedAttribute, LoadReport, SchemaErrorKind};
use crate::precompile::fixer::{sax_rewrite, Emit, Fixer, FixerError};

pub(crate) struct CxAxisIdShape;

impl Fixer for CxAxisIdShape {
    fn name(&self) -> &'static str {
        "cx_axis_id_shape"
    }

    fn applies_to(&self, part: &str) -> bool {
        part.starts_with("xl/charts/") && part.ends_with(".xml") && !part.contains("/_rels/")
    }

    fn rewrite(
        &self,
        xml: &[u8],
        part: &str,
        report: &mut LoadReport,
    ) -> Result<Option<Vec<u8>>, FixerError> {
        sax_rewrite(xml, part, report, |ctx, ev| match ev {
            Event::Empty(e) if e.name().as_ref() == b"cx:axisId" => {
                let val = read_attr(&e, b"val").unwrap_or_default();
                ctx.report.fixes.push(FixedAttribute {
                    part: ctx.part.to_string(),
                    ty: Some("cx:axisId".into()),
                    field: Some("val".into()),
                    value: Some(val.clone()),
                    occurrences: 1,
                    kind: SchemaErrorKind::UnexpectedTag,
                });
                Emit::Many(vec![
                    Event::Start(BytesStart::new("cx:axisId")),
                    Event::Text(BytesText::new(&val).into_owned()),
                    Event::End(BytesEnd::new("cx:axisId")),
                ])
            }
            other => Emit::Keep(other),
        })
    }
}

fn read_attr(e: &BytesStart<'_>, key: &[u8]) -> Option<String> {
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == key {
            return Some(
                std::str::from_utf8(&attr.value)
                    .ok()
                    .unwrap_or("")
                    .to_string(),
            );
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(xml: &str, part: &str) -> (Option<String>, LoadReport) {
        let mut report = LoadReport::default();
        let out = CxAxisIdShape
            .rewrite(xml.as_bytes(), part, &mut report)
            .expect("rewrite ok");
        (out.map(|b| String::from_utf8(b).expect("utf8")), report)
    }

    #[test]
    fn rewrites_attribute_to_text_child() {
        let (out, rep) = run(
            r#"<?xml version="1.0"?><cx:series><cx:axisId val="2"/></cx:series>"#,
            "xl/charts/chart1.xml",
        );
        let s = out.expect("rewritten");
        assert!(s.contains("<cx:axisId>2</cx:axisId>"), "{s}");
        assert!(!s.contains("<cx:axisId val"));
        assert_eq!(rep.fixes.len(), 1);
        let fix = &rep.fixes[0];
        assert_eq!(fix.part, "xl/charts/chart1.xml");
        assert_eq!(fix.field.as_deref(), Some("val"));
        assert_eq!(fix.value.as_deref(), Some("2"));
        assert_eq!(fix.occurrences, 1);
    }

    #[test]
    fn leaves_text_child_form_alone() {
        let (out, rep) = run(
            r#"<cx:series><cx:axisId>1</cx:axisId></cx:series>"#,
            "xl/charts/chart1.xml",
        );
        assert!(out.is_none(), "should be no-op");
        assert_eq!(rep.fixes.len(), 0);
    }

    #[test]
    fn does_not_touch_legacy_c_axis_id() {
        let (out, _) = run(
            r#"<c:plotArea><c:catAx><c:axisId val="1"/></c:catAx></c:plotArea>"#,
            "xl/charts/chart1.xml",
        );
        assert!(out.is_none(), "legacy c:axisId must be untouched");
    }

    #[test]
    fn applies_to_gates_on_chart_parts() {
        assert!(CxAxisIdShape.applies_to("xl/charts/chart1.xml"));
        assert!(!CxAxisIdShape.applies_to("xl/charts/_rels/chart1.xml.rels"));
        assert!(!CxAxisIdShape.applies_to("xl/worksheets/sheet1.xml"));
        assert!(!CxAxisIdShape.applies_to("xl/charts/chart1.xml.rels"));
    }

    #[test]
    fn multiple_occurrences_each_recorded() {
        let (out, rep) = run(
            r#"<r><cx:axisId val="1"/><cx:axisId val="2"/></r>"#,
            "xl/charts/chart1.xml",
        );
        let s = out.expect("rewritten");
        assert!(s.contains("<cx:axisId>1</cx:axisId>"));
        assert!(s.contains("<cx:axisId>2</cx:axisId>"));
        assert_eq!(rep.fixes.len(), 2);
        assert_eq!(rep.fixes[0].value.as_deref(), Some("1"));
        assert_eq!(rep.fixes[1].value.as_deref(), Some("2"));
    }
}
