use quick_xml::events::{BytesEnd, BytesStart, Event};

use crate::error::{FixedAttribute, LoadReport, SchemaErrorKind};
use crate::precompile::fixer::{sax_rewrite, Emit, Fixer, FixerError};

const AC: &[u8] = b"mc:AlternateContent";
const CHOICE: &[u8] = b"mc:Choice";
const FALLBACK: &[u8] = b"mc:Fallback";

pub(crate) struct AlternateContentUnfolder;

impl Fixer for AlternateContentUnfolder {
    fn name(&self) -> &'static str {
        "mc_alternate_content_unfold"
    }

    fn rewrite(
        &self,
        xml: &[u8],
        part: &str,
        report: &mut LoadReport,
    ) -> Result<Option<Vec<u8>>, FixerError> {
        let mut state: State = State::Pass;

        sax_rewrite(xml, part, report, |ctx, ev| match (&mut state, ev) {
            (State::Pass, Event::Start(e)) if e.name().as_ref() == AC => {
                state = State::Buffering {
                    depth: 1,
                    buf: Vec::with_capacity(16),
                };
                Emit::Drop
            }
            (State::Pass, Event::Empty(e)) if e.name().as_ref() == AC => {
                ctx.report.fixes.push(FixedAttribute {
                    part: ctx.part.to_string(),
                    ty: Some(qname_string(&e)),
                    field: Some("element".into()),
                    value: Some("empty".into()),
                    occurrences: 1,
                    kind: SchemaErrorKind::UnexpectedTag,
                });
                Emit::Drop
            }
            (State::Pass, other) => Emit::Keep(other),

            (State::Buffering { depth, buf }, ev) => {
                let is_ac_start = matches!(&ev, Event::Start(e) if e.name().as_ref() == AC);
                let is_ac_end = matches!(&ev, Event::End(e) if e.name().as_ref() == AC);

                if is_ac_start {
                    *depth += 1;
                    buf.push(into_owned(ev));
                    return Emit::Drop;
                }
                if is_ac_end {
                    *depth -= 1;
                    if *depth > 0 {
                        buf.push(into_owned(ev));
                        return Emit::Drop;
                    }

                    let buf_taken = std::mem::take(buf);
                    let inner = pick_branch(buf_taken);
                    state = State::Pass;
                    ctx.report.fixes.push(FixedAttribute {
                        part: ctx.part.to_string(),
                        ty: Some("mc:AlternateContent".into()),
                        field: Some("element".into()),
                        value: Some("unfolded".into()),
                        occurrences: 1,
                        kind: SchemaErrorKind::UnexpectedTag,
                    });
                    return Emit::Many(inner);
                }
                buf.push(into_owned(ev));
                Emit::Drop
            }
        })
    }
}

enum State {
    Pass,
    Buffering {
        depth: usize,
        buf: Vec<Event<'static>>,
    },
}

fn pick_branch(buf: Vec<Event<'static>>) -> Vec<Event<'static>> {
    if let Some(inner) = extract_branch(&buf, CHOICE) {
        return inner;
    }
    if let Some(inner) = extract_branch(&buf, FALLBACK) {
        return inner;
    }
    Vec::new()
}

fn extract_branch(buf: &[Event<'static>], tag: &[u8]) -> Option<Vec<Event<'static>>> {
    let mut iter = buf.iter().enumerate();
    let (open_idx, _) = iter.find(|(_, ev)| match ev {
        Event::Start(e) => e.name().as_ref() == tag,
        Event::Empty(e) if e.name().as_ref() == tag => true,
        _ => false,
    })?;

    if matches!(&buf[open_idx], Event::Empty(_)) {
        return Some(Vec::new());
    }

    let mut depth = 1usize;
    for (i, ev) in buf.iter().enumerate().skip(open_idx + 1) {
        match ev {
            Event::Start(e) if e.name().as_ref() == tag => depth += 1,
            Event::End(e) if e.name().as_ref() == tag => {
                depth -= 1;
                if depth == 0 {
                    return Some(buf[open_idx + 1..i].to_vec());
                }
            }
            _ => {}
        }
    }

    Some(buf[open_idx + 1..].to_vec())
}

fn into_owned(ev: Event<'_>) -> Event<'static> {
    match ev {
        Event::Start(e) => Event::Start(e.into_owned()),
        Event::End(e) => {
            let name = std::str::from_utf8(e.name().as_ref())
                .unwrap_or("")
                .to_string();
            Event::End(BytesEnd::new(name))
        }
        Event::Empty(e) => Event::Empty(e.into_owned()),
        Event::Text(e) => Event::Text(e.into_owned()),
        Event::CData(e) => Event::CData(e.into_owned()),
        Event::Comment(e) => Event::Comment(e.into_owned()),
        Event::Decl(e) => Event::Decl(e.into_owned()),
        Event::PI(e) => Event::PI(e.into_owned()),
        Event::DocType(e) => Event::DocType(e.into_owned()),
        Event::GeneralRef(e) => Event::GeneralRef(e.into_owned()),
        Event::Eof => Event::Eof,
    }
}

fn qname_string(e: &BytesStart<'_>) -> String {
    String::from_utf8_lossy(e.name().as_ref()).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(xml: &str) -> (Option<String>, LoadReport) {
        let mut report = LoadReport::default();
        let out = AlternateContentUnfolder
            .rewrite(xml.as_bytes(), "xl/worksheets/sheet1.xml", &mut report)
            .expect("ok");
        (out.map(|b| String::from_utf8(b).unwrap()), report)
    }

    #[test]
    fn picks_choice_when_present() {
        let (out, rep) = run(
            r#"<r><mc:AlternateContent><mc:Choice Requires="x14"><x14:foo/></mc:Choice><mc:Fallback><legacy/></mc:Fallback></mc:AlternateContent></r>"#,
        );
        let s = out.expect("rewritten");
        assert!(s.contains("<x14:foo"), "{s}");
        assert!(!s.contains("legacy"), "{s}");
        assert!(!s.contains("mc:"), "{s}");
        assert_eq!(rep.fixes.len(), 1);
    }

    #[test]
    fn falls_back_when_no_choice() {
        let (out, _) = run(
            r#"<r><mc:AlternateContent><mc:Fallback><legacy/></mc:Fallback></mc:AlternateContent></r>"#,
        );
        let s = out.expect("rewritten");
        assert!(s.contains("<legacy"), "{s}");
        assert!(!s.contains("mc:"));
    }

    #[test]
    fn drops_empty_self_closing_wrapper() {
        let (out, rep) = run(r#"<r><mc:AlternateContent/></r>"#);
        let s = out.expect("rewritten");
        assert!(!s.contains("mc:"), "{s}");
        assert_eq!(rep.fixes.len(), 1);
        assert_eq!(rep.fixes[0].value.as_deref(), Some("empty"));
    }

    #[test]
    fn passthrough_when_no_alternate_content() {
        let (out, _) = run(r#"<r><a/><b>x</b></r>"#);
        assert!(out.is_none());
    }

    #[test]
    fn preserves_siblings_around_wrapper() {
        let (out, _) = run(
            r#"<r><before/><mc:AlternateContent><mc:Choice><inner/></mc:Choice></mc:AlternateContent><after/></r>"#,
        );
        let s = out.expect("rewritten");
        assert!(s.contains("<before/>"), "{s}");
        assert!(s.contains("<inner/>"), "{s}");
        assert!(s.contains("<after/>"), "{s}");
    }

    #[test]
    fn multiple_wrappers_in_one_doc() {
        let (out, rep) = run(r#"<r>
              <mc:AlternateContent><mc:Choice><a/></mc:Choice></mc:AlternateContent>
              <mc:AlternateContent><mc:Fallback><b/></mc:Fallback></mc:AlternateContent>
            </r>"#);
        let s = out.expect("rewritten");
        assert!(s.contains("<a/>"));
        assert!(s.contains("<b/>"));
        assert!(!s.contains("mc:"));
        assert_eq!(rep.fixes.len(), 2);
    }

    #[test]
    fn keeps_nested_content_inside_choice() {
        let (out, _) = run(
            r#"<r><mc:AlternateContent><mc:Choice><outer><inner attr="v">text</inner></outer></mc:Choice></mc:AlternateContent></r>"#,
        );
        let s = out.expect("rewritten");
        assert!(s.contains("<outer>"));
        assert!(s.contains(r#"<inner attr="v">text</inner>"#));
        assert!(s.contains("</outer>"));
    }
}
