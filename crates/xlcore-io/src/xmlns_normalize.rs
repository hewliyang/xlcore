use std::io::{Cursor, Read, Write};

fn canonical_prefix(uri: &str) -> Option<&'static str> {
    match uri {
        "http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments" => Some("xltc"),
        "http://schemas.microsoft.com/office/spreadsheetml/2020/threadedcomments2" => Some("xltc2"),
        _ => None,
    }
}

fn is_target_part(name: &str) -> bool {
    if !name.ends_with(".xml") {
        return false;
    }

    if name.starts_with("xl/persons/") || name.starts_with("xl/threadedComments/") {
        return true;
    }

    if name.starts_with("xl/worksheets/") && !name.contains("/_rels/") {
        return true;
    }

    if name.starts_with("xl/drawings/") && !name.contains("/_rels/") {
        return true;
    }

    if name.starts_with("xl/charts/") && !name.contains("/_rels/") {
        return true;
    }
    false
}

pub(crate) fn normalize_xlsx(bytes: Vec<u8>) -> anyhow::Result<Vec<u8>> {
    let mut zip = match zip::ZipArchive::new(Cursor::new(&bytes)) {
        Ok(z) => z,

        Err(_) => return Ok(bytes),
    };

    let mut rewrites: std::collections::HashMap<String, Vec<u8>> = std::collections::HashMap::new();
    let names: Vec<String> = (0..zip.len())
        .map(|i| zip.by_index(i).map(|f| f.name().to_string()))
        .collect::<Result<_, _>>()?;
    for name in &names {
        if !is_target_part(name) {
            continue;
        }
        let mut f = zip.by_name(name)?;
        let mut data = Vec::new();
        f.read_to_end(&mut data)?;
        if let Some(new) = rewrite_xml(&data) {
            rewrites.insert(name.clone(), new);
        }
    }
    if rewrites.is_empty() {
        return Ok(bytes);
    }

    let mut out = Vec::with_capacity(bytes.len());
    {
        let mut writer = zip::ZipWriter::new(Cursor::new(&mut out));
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            let name = entry.name().to_string();
            if entry.is_dir() {
                writer.add_directory(name, opts)?;
                continue;
            }
            writer.start_file(name.clone(), opts)?;
            if let Some(replacement) = rewrites.remove(&name) {
                writer.write_all(&replacement)?;
            } else {
                let mut data = Vec::new();
                entry.read_to_end(&mut data)?;
                writer.write_all(&data)?;
            }
        }
        writer.finish()?;
    }
    Ok(out)
}

fn rewrite_xml(xml: &[u8]) -> Option<Vec<u8>> {
    let s = std::str::from_utf8(xml).ok()?;
    let mut out = s.to_string();
    let mut changed = false;

    for (old_prefix, uri) in scan_xmlns_prefixes(s) {
        let Some(canonical) = canonical_prefix(uri) else {
            continue;
        };
        if old_prefix == canonical {
            continue;
        }

        out = out.replace(
            &format!("xmlns:{}=", old_prefix),
            &format!("xmlns:{}=", canonical),
        );

        for (left, right) in [
            (format!("<{}:", old_prefix), format!("<{}:", canonical)),
            (format!("</{}:", old_prefix), format!("</{}:", canonical)),
            (format!(" {}:", old_prefix), format!(" {}:", canonical)),
        ] {
            out = out.replace(&left, &right);
        }
        changed = true;
    }

    if rewrite_x14_databar_color(&mut out) {
        changed = true;
    }

    if unfold_mc_alternate_content(&mut out) {
        changed = true;
    }

    if rewrite_cx_axis_id(&mut out) {
        changed = true;
    }

    if changed {
        Some(out.into_bytes())
    } else {
        None
    }
}

fn rewrite_x14_databar_color(s: &mut String) -> bool {
    const OPEN: &str = "<x14:dataBar";
    const CLOSE: &str = "</x14:dataBar>";
    if !s.contains(OPEN) {
        return false;
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s.as_str();
    let mut changed = false;
    while let Some(open_idx) = rest.find(OPEN) {
        out.push_str(&rest[..open_idx]);
        let after_open = &rest[open_idx..];
        let close_rel = match after_open.find(CLOSE) {
            Some(i) => i + CLOSE.len(),

            None => {
                out.push_str(after_open);
                rest = "";
                break;
            }
        };
        let block = &after_open[..close_rel];

        let mut rewritten = String::with_capacity(block.len());
        let mut bi = 0usize;
        let bytes = block.as_bytes();
        let open_tag = b"<x14:color";
        let close_tag = b"</x14:color>";
        while bi < bytes.len() {
            if bytes[bi..].starts_with(close_tag) {
                rewritten.push_str("</x14:fillColor>");
                bi += close_tag.len();
                changed = true;
                continue;
            }
            if bytes[bi..].starts_with(open_tag) {
                let after = bytes.get(bi + open_tag.len()).copied();

                if matches!(
                    after,
                    Some(b' ') | Some(b'/') | Some(b'>') | Some(b'\t') | Some(b'\n') | Some(b'\r')
                ) {
                    rewritten.push_str("<x14:fillColor");
                    bi += open_tag.len();
                    changed = true;
                    continue;
                }
            }

            let ch_end = block[bi..]
                .char_indices()
                .nth(1)
                .map(|(o, _)| bi + o)
                .unwrap_or(block.len());
            rewritten.push_str(&block[bi..ch_end]);
            bi = ch_end;
        }
        out.push_str(&rewritten);
        rest = &after_open[close_rel..];
    }
    out.push_str(rest);
    if changed {
        *s = out;
    }
    changed
}

fn unfold_mc_alternate_content(s: &mut String) -> bool {
    const OPEN_PREFIX: &str = "<mc:AlternateContent";
    const CLOSE: &str = "</mc:AlternateContent>";
    if !s.contains(OPEN_PREFIX) {
        return false;
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s.as_str();
    let mut changed = false;
    while let Some(open_idx) = rest.find(OPEN_PREFIX) {
        out.push_str(&rest[..open_idx]);
        let after_open = &rest[open_idx..];

        let tag_end = match after_open.find('>') {
            Some(i) => i,
            None => {
                out.push_str(after_open);
                rest = "";
                break;
            }
        };
        if after_open.as_bytes()[tag_end.saturating_sub(1)] == b'/' {
            rest = &after_open[tag_end + 1..];
            changed = true;
            continue;
        }
        let close_rel = match after_open.find(CLOSE) {
            Some(i) => i,
            None => {
                out.push_str(after_open);
                rest = "";
                break;
            }
        };
        let inner = &after_open[tag_end + 1..close_rel];

        let chosen = extract_mc_container(inner, "mc:Choice")
            .or_else(|| extract_mc_container(inner, "mc:Fallback"))
            .unwrap_or("");
        out.push_str(chosen);
        rest = &after_open[close_rel + CLOSE.len()..];
        changed = true;
    }
    out.push_str(rest);
    if changed {
        *s = out;
    }
    changed
}

fn extract_mc_container<'a>(s: &'a str, tag: &str) -> Option<&'a str> {
    let open_token = format!("<{}", tag);
    let close_token = format!("</{}>", tag);
    let open_idx = s.find(&open_token)?;
    let after_open = &s[open_idx..];

    let tag_end = after_open.find('>')?;
    if after_open.as_bytes()[tag_end.saturating_sub(1)] == b'/' {
        return Some("");
    }
    let close_rel = after_open.find(&close_token)?;
    Some(&after_open[tag_end + 1..close_rel])
}

fn rewrite_cx_axis_id(s: &mut String) -> bool {
    const TAG: &str = "<cx:axisId";
    if !s.contains(TAG) {
        return false;
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s.as_str();
    let mut changed = false;
    while let Some(idx) = rest.find(TAG) {
        out.push_str(&rest[..idx]);
        let after = &rest[idx..];

        let end_rel = match after.find('>') {
            Some(i) => i,
            None => {
                out.push_str(after);
                rest = "";
                break;
            }
        };
        let element = &after[..=end_rel];
        let after_element = &after[end_rel + 1..];

        if !element.ends_with("/>") {
            out.push_str(element);
            rest = after_element;
            continue;
        }

        let val = extract_attr(element, "val").unwrap_or("");
        out.push_str("<cx:axisId>");
        out.push_str(val);
        out.push_str("</cx:axisId>");
        rest = after_element;
        changed = true;
    }
    out.push_str(rest);
    if changed {
        *s = out;
    }
    changed
}

fn extract_attr<'a>(element: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("{}=", name);
    let mut search = element;
    while let Some(rel) = search.find(&needle) {
        if rel > 0 {
            let prev = search.as_bytes()[rel - 1];
            if !(prev.is_ascii_whitespace() || prev == b'<') {
                search = &search[rel + needle.len()..];
                continue;
            }
        }
        let after = &search[rel + needle.len()..];
        let quote = after.as_bytes().first().copied()?;
        if quote != b'"' && quote != b'\'' {
            return None;
        }
        let inner = &after[1..];
        let end = inner.find(quote as char)?;
        return Some(&inner[..end]);
    }
    None
}

fn scan_xmlns_prefixes(s: &str) -> Vec<(&str, &str)> {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while let Some(rel) = s[i..].find("xmlns:") {
        let start = i + rel + "xmlns:".len();

        let mut end = start;
        while end < bytes.len() {
            let c = bytes[end];
            if c == b'=' || c.is_ascii_whitespace() {
                break;
            }
            end += 1;
        }
        let prefix = &s[start..end];

        let mut p = end;
        while p < bytes.len() && bytes[p].is_ascii_whitespace() {
            p += 1;
        }
        if p >= bytes.len() || bytes[p] != b'=' {
            i = end;
            continue;
        }
        p += 1;
        while p < bytes.len() && bytes[p].is_ascii_whitespace() {
            p += 1;
        }
        if p >= bytes.len() {
            break;
        }
        let quote = bytes[p];
        if quote != b'"' && quote != b'\'' {
            i = p;
            continue;
        }
        p += 1;
        let val_start = p;
        while p < bytes.len() && bytes[p] != quote {
            p += 1;
        }
        if p >= bytes.len() {
            break;
        }
        let uri = &s[val_start..p];
        if !prefix.is_empty() {
            out.push((prefix, uri));
        }
        i = p + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_x18tc_to_xltc() {
        let xml = br#"<?xml version="1.0"?><x18tc:personList xmlns:x18tc="http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments"><x18tc:person id="x"/></x18tc:personList>"#;
        let out = rewrite_xml(xml).expect("should rewrite");
        let s = std::str::from_utf8(&out).unwrap();
        assert!(s.contains("xmlns:xltc="), "{s}");
        assert!(s.contains("<xltc:personList"));
        assert!(s.contains("<xltc:person "));
        assert!(s.contains("</xltc:personList>"));
        assert!(!s.contains("x18tc"));
    }

    #[test]
    fn leaves_canonical_prefix_alone() {
        let xml = br#"<xltc:personList xmlns:xltc="http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments"/>"#;
        assert!(rewrite_xml(xml).is_none());
    }

    #[test]
    fn rewrites_x14_color_inside_databar() {
        let xml = br#"<?xml version="1.0"?><worksheet><extLst><ext><x14:dataBar minLength="0" maxLength="100"><x14:color rgb="FF4472C4"/><x14:cfvo type="min"/></x14:dataBar></ext></extLst></worksheet>"#;
        let out = rewrite_xml(xml).expect("should rewrite");
        let s = std::str::from_utf8(&out).unwrap();
        assert!(s.contains("<x14:fillColor rgb=\"FF4472C4\"/>"), "{s}");
        assert!(!s.contains("<x14:color "));
    }

    #[test]
    fn leaves_x14_color_outside_databar_alone() {
        let xml = br#"<root><x14:color rgb="FFAA0000"/></root>"#;
        assert!(rewrite_xml(xml).is_none());
    }

    #[test]
    fn does_not_match_color_scale_substring() {
        let xml = br#"<x14:dataBar><x14:colorScale/></x14:dataBar>"#;

        assert!(rewrite_xml(xml).is_none());
    }

    #[test]
    fn rewrites_cx_axis_id_attribute_to_text_child() {
        let xml = br#"<?xml version="1.0"?><cx:series><cx:axisId val="2"/></cx:series>"#;
        let out = rewrite_xml(xml).expect("should rewrite");
        let s = std::str::from_utf8(&out).unwrap();
        assert!(s.contains("<cx:axisId>2</cx:axisId>"), "{s}");
        assert!(!s.contains("<cx:axisId val"));
    }

    #[test]
    fn leaves_cx_axis_id_text_child_alone() {
        let xml = br#"<cx:series><cx:axisId>1</cx:axisId></cx:series>"#;
        assert!(rewrite_xml(xml).is_none());
    }

    #[test]
    fn ignores_unknown_namespace() {
        let xml = br#"<foo:bar xmlns:foo="http://example.com/unknown"/>"#;
        assert!(rewrite_xml(xml).is_none());
    }
}
