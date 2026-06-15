use std::collections::HashMap;

use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesEnd, BytesStart, Event};

use crate::error::{FixedAttribute, LoadReport, SchemaErrorKind};
use crate::precompile::fixer::{sax_rewrite, Emit, Fixer, FixerError};

fn canonical_prefix(uri: &[u8]) -> Option<&'static str> {
    match uri {
        b"http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments" => Some("xltc"),
        b"http://schemas.microsoft.com/office/spreadsheetml/2020/threadedcomments2" => {
            Some("xltc2")
        }
        _ => None,
    }
}

pub(crate) struct PrefixCanonicalizer;

impl Fixer for PrefixCanonicalizer {
    fn name(&self) -> &'static str {
        "prefix_canonicalize"
    }

    fn applies_to(&self, part: &str) -> bool {
        if !part.ends_with(".xml") {
            return false;
        }
        if part.contains("/_rels/") {
            return false;
        }
        part.starts_with("xl/persons/")
            || part.starts_with("xl/threadedComments/")
            || part.starts_with("xl/worksheets/")
            || part.starts_with("xl/drawings/")
            || part.starts_with("xl/charts/")
    }

    fn rewrite(
        &self,
        xml: &[u8],
        part: &str,
        report: &mut LoadReport,
    ) -> Result<Option<Vec<u8>>, FixerError> {
        let mut prefix_map: HashMap<Vec<u8>, &'static str> = HashMap::new();

        sax_rewrite(xml, part, report, |ctx, ev| match ev {
            Event::Start(e) => {
                discover_prefixes(&e, &mut prefix_map, ctx);
                match rewrite_start(&e, &prefix_map) {
                    Some(new) => Emit::Replace {
                        event: Event::Start(new),
                        changed: true,
                    },
                    None => Emit::Keep(Event::Start(e)),
                }
            }
            Event::Empty(e) => {
                discover_prefixes(&e, &mut prefix_map, ctx);
                match rewrite_start(&e, &prefix_map) {
                    Some(new) => Emit::Replace {
                        event: Event::Empty(new),
                        changed: true,
                    },
                    None => Emit::Keep(Event::Empty(e)),
                }
            }
            Event::End(e) => {
                let name = e.name();
                match rename_qname(name.as_ref(), &prefix_map) {
                    Some(new) => Emit::Replace {
                        event: Event::End(BytesEnd::new(new)),
                        changed: true,
                    },
                    None => Emit::Keep(Event::End(e)),
                }
            }
            other => Emit::Keep(other),
        })
    }
}

fn discover_prefixes(
    e: &BytesStart<'_>,
    map: &mut HashMap<Vec<u8>, &'static str>,
    ctx: &mut crate::precompile::fixer::Ctx<'_>,
) {
    for attr in e.attributes().flatten() {
        let key = attr.key.as_ref();
        let Some(prefix) = key.strip_prefix(b"xmlns:") else {
            continue;
        };
        let uri = attr.value.as_ref();
        let Some(canon) = canonical_prefix(uri) else {
            continue;
        };
        if prefix == canon.as_bytes() {
            continue;
        }

        if map.contains_key(prefix) {
            continue;
        }
        map.insert(prefix.to_vec(), canon);
        ctx.report.fixes.push(FixedAttribute {
            part: ctx.part.to_string(),
            ty: Some("xmlns".into()),
            field: Some(std::str::from_utf8(prefix).unwrap_or("?").to_string()),
            value: Some(std::str::from_utf8(uri).unwrap_or("?").to_string()),
            occurrences: 1,
            kind: SchemaErrorKind::UnexpectedTag,
        });
    }
}

fn rewrite_start(
    src: &BytesStart<'_>,
    map: &HashMap<Vec<u8>, &'static str>,
) -> Option<BytesStart<'static>> {
    if map.is_empty() {
        return None;
    }

    let tag_renamed = rename_qname(src.name().as_ref(), map);
    let mut new_attrs: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    let mut any_attr_renamed = false;
    for attr in src.attributes().flatten() {
        let key = attr.key.as_ref();
        let new_key = rename_attr_key(key, map);
        if let Some(k) = &new_key {
            any_attr_renamed = true;
            new_attrs.push((k.clone().into_bytes(), attr.value.into_owned()));
        } else {
            new_attrs.push((key.to_vec(), attr.value.into_owned()));
        }
    }
    if tag_renamed.is_none() && !any_attr_renamed {
        return None;
    }

    let name = tag_renamed.unwrap_or_else(|| {
        std::str::from_utf8(src.name().as_ref())
            .unwrap_or("")
            .to_string()
    });
    let mut out = BytesStart::new(name);
    for (k, v) in new_attrs {
        out.push_attribute(Attribute {
            key: quick_xml::name::QName(&k),
            value: std::borrow::Cow::Owned(v),
        });
    }
    Some(out)
}

fn rename_qname(qname: &[u8], map: &HashMap<Vec<u8>, &'static str>) -> Option<String> {
    let (prefix, local) = split_qname(qname);
    let prefix = prefix?;
    let canon = map.get(prefix)?;
    let mut s = String::with_capacity(canon.len() + 1 + local.len());
    s.push_str(canon);
    s.push(':');
    s.push_str(std::str::from_utf8(local).ok()?);
    Some(s)
}

fn rename_attr_key(key: &[u8], map: &HashMap<Vec<u8>, &'static str>) -> Option<String> {
    if let Some(prefix) = key.strip_prefix(b"xmlns:") {
        let canon = map.get(prefix)?;
        return Some(format!("xmlns:{canon}"));
    }

    rename_qname(key, map)
}

fn split_qname(q: &[u8]) -> (Option<&[u8]>, &[u8]) {
    match q.iter().position(|&b| b == b':') {
        Some(i) => (Some(&q[..i]), &q[i + 1..]),
        None => (None, q),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(xml: &str, part: &str) -> (Option<String>, LoadReport) {
        let mut report = LoadReport::default();
        let out = PrefixCanonicalizer
            .rewrite(xml.as_bytes(), part, &mut report)
            .expect("ok");
        (out.map(|b| String::from_utf8(b).unwrap()), report)
    }

    #[test]
    fn rewrites_x18tc_to_xltc_on_root() {
        let (out, rep) = run(
            r#"<?xml version="1.0"?><x18tc:personList xmlns:x18tc="http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments"><x18tc:person id="x"/></x18tc:personList>"#,
            "xl/persons/personList.xml",
        );
        let s = out.expect("rewritten");
        assert!(s.contains("xmlns:xltc="), "{s}");
        assert!(s.contains("<xltc:personList"));
        assert!(s.contains("<xltc:person "));
        assert!(s.contains("</xltc:personList>"));
        assert!(!s.contains("x18tc"));
        assert_eq!(rep.fixes.len(), 1);
        assert_eq!(rep.fixes[0].field.as_deref(), Some("x18tc"));
    }

    #[test]
    fn leaves_canonical_prefix_alone() {
        let (out, _) = run(
            r#"<xltc:personList xmlns:xltc="http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments"/>"#,
            "xl/persons/personList.xml",
        );
        assert!(out.is_none());
    }

    #[test]
    fn ignores_unknown_namespace() {
        let (out, _) = run(
            r#"<foo:bar xmlns:foo="http://example.com/unknown"/>"#,
            "xl/persons/x.xml",
        );
        assert!(out.is_none());
    }

    #[test]
    fn applies_to_gates_correctly() {
        assert!(PrefixCanonicalizer.applies_to("xl/persons/personList.xml"));
        assert!(PrefixCanonicalizer.applies_to("xl/threadedComments/threadedComment1.xml"));
        assert!(!PrefixCanonicalizer.applies_to("xl/sharedStrings.xml"));
        assert!(!PrefixCanonicalizer.applies_to("xl/persons/_rels/x.xml.rels"));
    }

    #[test]
    fn renames_prefixed_attribute_keys() {
        let (out, _) = run(
            r#"<root xmlns:x18tc="http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments"><node x18tc:id="abc"/></root>"#,
            "xl/threadedComments/x.xml",
        );
        let s = out.expect("rewritten");
        assert!(s.contains(r#"xltc:id="abc""#), "{s}");
        assert!(!s.contains("x18tc:id"));
    }
}
