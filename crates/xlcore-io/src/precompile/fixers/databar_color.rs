use quick_xml::events::{BytesEnd, BytesStart, Event};

use crate::error::{FixedAttribute, LoadReport, SchemaErrorKind};
use crate::precompile::fixer::{sax_rewrite, Emit, Fixer, FixerError};

pub(crate) struct DataBarColorRename;

const FROM: &[u8] = b"x14:color";
const TO: &str = "x14:fillColor";
const PARENT: &[u8] = b"x14:dataBar";

impl Fixer for DataBarColorRename {
    fn name(&self) -> &'static str {
        "databar_color_rename"
    }

    fn rewrite(
        &self,
        xml: &[u8],
        part: &str,
        report: &mut LoadReport,
    ) -> Result<Option<Vec<u8>>, FixerError> {
        sax_rewrite(xml, part, report, |ctx, ev| match ev {
            Event::Start(e) if e.name().as_ref() == FROM && ctx.in_element(PARENT) => {
                ctx.report.fixes.push(FixedAttribute {
                    part: ctx.part.to_string(),
                    ty: Some("x14:dataBar".into()),
                    field: Some("element".into()),
                    value: Some("x14:color".into()),
                    occurrences: 1,
                    kind: SchemaErrorKind::UnexpectedTag,
                });
                Emit::Replace {
                    event: Event::Start(rename(&e)),
                    changed: true,
                }
            }
            Event::Empty(e) if e.name().as_ref() == FROM && ctx.in_element(PARENT) => {
                ctx.report.fixes.push(FixedAttribute {
                    part: ctx.part.to_string(),
                    ty: Some("x14:dataBar".into()),
                    field: Some("element".into()),
                    value: Some("x14:color".into()),
                    occurrences: 1,
                    kind: SchemaErrorKind::UnexpectedTag,
                });
                Emit::Replace {
                    event: Event::Empty(rename(&e)),
                    changed: true,
                }
            }

            Event::End(e) if e.name().as_ref() == FROM && ctx.in_element(PARENT) => Emit::Replace {
                event: Event::End(BytesEnd::new(TO)),
                changed: true,
            },
            other => Emit::Keep(other),
        })
    }
}

fn rename(src: &BytesStart<'_>) -> BytesStart<'static> {
    let mut out = BytesStart::new(TO);
    for attr in src.attributes().flatten() {
        out.push_attribute(attr);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(xml: &str) -> (Option<String>, LoadReport) {
        let mut report = LoadReport::default();
        let out = DataBarColorRename
            .rewrite(xml.as_bytes(), "xl/worksheets/sheet1.xml", &mut report)
            .expect("ok");
        (out.map(|b| String::from_utf8(b).unwrap()), report)
    }

    #[test]
    fn renames_self_closing_color_inside_databar() {
        let (out, rep) = run(
            r#"<worksheet><extLst><ext><x14:dataBar minLength="0"><x14:color rgb="FF4472C4"/><x14:cfvo type="min"/></x14:dataBar></ext></extLst></worksheet>"#,
        );
        let s = out.expect("rewritten");
        assert!(s.contains(r#"<x14:fillColor rgb="FF4472C4"/>"#), "{s}");
        assert!(!s.contains("<x14:color "));
        assert_eq!(rep.fixes.len(), 1);
        assert_eq!(rep.fixes[0].part, "xl/worksheets/sheet1.xml");
    }

    #[test]
    fn renames_paired_open_close_inside_databar() {
        let (out, rep) =
            run(r#"<x14:dataBar><x14:color><x14:rgb val="FF000000"/></x14:color></x14:dataBar>"#);
        let s = out.expect("rewritten");
        assert!(s.contains("<x14:fillColor>"), "{s}");
        assert!(s.contains("</x14:fillColor>"), "{s}");
        assert!(!s.contains("x14:color"));

        assert_eq!(rep.fixes.len(), 1);
    }

    #[test]
    fn leaves_color_outside_databar_alone() {
        let (out, _) = run(r#"<root><x14:color rgb="FFAA0000"/></root>"#);
        assert!(out.is_none(), "must not touch x14:color outside dataBar");
    }

    #[test]
    fn does_not_match_color_scale_substring() {

        let (out, _) = run(r#"<x14:dataBar><x14:colorScale/></x14:dataBar>"#);
        assert!(out.is_none(), "colorScale must survive");
    }

    #[test]
    fn nested_databar_color_is_still_renamed() {

        let (out, _) =
            run(r#"<x14:dataBar><wrap><x14:color rgb="FF0000FF"/></wrap></x14:dataBar>"#);
        let s = out.expect("rewritten");
        assert!(s.contains("<x14:fillColor"), "{s}");
    }
}
