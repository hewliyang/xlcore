//! Pre-parse normalization for OOXML that is schema-valid but incompatible
//! with `ooxmlsdk`'s literal-name parser.
//!
//! Two kinds of fixups today:
//!
//! 1. **Non-canonical namespace prefixes.** ooxmlsdk matches elements by
//!    literal prefix (e.g. it expects `xltc:personList` rather than
//!    resolving via namespace URI). Google Sheets binds the threaded
//!    comments namespace to `x18tc:` instead of `xltc:`, which raises
//!    `unexpected tag while parsing PersonList`. We normalize prefixes
//!    inside `xl/persons/*.xml` and `xl/threadedComments/*.xml`.
//!
//! 2. **`<x14:color>` inside `<x14:dataBar>`.** Excel desktop emits
//!    `<x14:color rgb="..."/>` as the bar fill color, but ooxmlsdk's
//!    `x14:DataBar` schema only accepts the newer `x14:fillColor` /
//!    `x14:borderColor` / `x14:negativeFillColor` / `x14:negativeBorderColor`
//!    / `x14:axisColor` slots. The plain `<x14:color>` child fails the
//!    `known child` check (its prefix matches, so it isn't skipped as
//!    foreign) and aborts the whole parse. We rewrite the tag name to
//!    `x14:fillColor` inside `<x14:dataBar>` blocks of worksheet XML.
//!
//! If no rewrite is needed, the original bytes are returned untouched
//! (no zip re-pack).
use std::io::{Cursor, Read, Write};

/// Map a namespace URI to the prefix that ooxmlsdk hard-codes for it.
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
    // Prefix-rebinding affected parts (fixup #1).
    if name.starts_with("xl/persons/") || name.starts_with("xl/threadedComments/") {
        return true;
    }
    // Worksheet parts may carry the `<x14:dataBar><x14:color/></...>` shape.
    // `rewrite_xml` is a no-op when no covered dataBar block is present.
    if name.starts_with("xl/worksheets/") && !name.contains("/_rels/") {
        return true;
    }
    // Drawing parts may carry `<mc:AlternateContent>` blocks (chartEx /
    // 2010+ shape extensions). ooxmlsdk's `mce` processing only
    // flattens unknown other-children, but `<a:graphic>` / `<xdr:graphicFrame>`
    // — the typical AlternateContent payload — are slotted into typed
    // choice fields that never see those replacements, so the chartEx
    // graphic silently disappears. We textually unfold the Choice
    // content (preferring `Requires="cx1"`, falling back to the
    // Fallback) here, then let ooxmlsdk parse the result as a plain
    // graphicFrame.
    if name.starts_with("xl/drawings/") && !name.contains("/_rels/") {
        return true;
    }
    false
}

/// Return a normalized xlsx zip when a covered part needs rewriting; otherwise
/// return the original bytes.
pub(crate) fn normalize_xlsx(bytes: Vec<u8>) -> anyhow::Result<Vec<u8>> {
    let mut zip = match zip::ZipArchive::new(Cursor::new(&bytes)) {
        Ok(z) => z,
        // Not a zip; let ooxmlsdk produce its native error.
        Err(_) => return Ok(bytes),
    };

    // First pass: scan candidate parts, collect rewrites.
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

    // Second pass: repack the zip with the rewritten parts substituted.
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

/// Rewrite a single XML payload. Returns `Some(new_bytes)` when at least one
/// rebinding was applied, or `None` if everything was already canonical.
fn rewrite_xml(xml: &[u8]) -> Option<Vec<u8>> {
    let s = std::str::from_utf8(xml).ok()?;
    let mut out = s.to_string();
    let mut changed = false;

    // Find every `xmlns:PFX="URI"` declaration; for each URI we care about,
    // if the prefix isn't canonical, rewrite the document.
    for (old_prefix, uri) in scan_xmlns_prefixes(s) {
        let Some(canonical) = canonical_prefix(uri) else {
            continue;
        };
        if old_prefix == canonical {
            continue;
        }
        // Replace the binding itself.
        out = out.replace(
            &format!("xmlns:{}=", old_prefix),
            &format!("xmlns:{}=", canonical),
        );
        // Replace prefix usage in element/attribute names. Anchor each
        // replacement to a syntactic position so we don't mangle attribute
        // values that happen to contain `OLD:` as a substring.
        for (left, right) in [
            (format!("<{}:", old_prefix), format!("<{}:", canonical)),
            (format!("</{}:", old_prefix), format!("</{}:", canonical)),
            (format!(" {}:", old_prefix), format!(" {}:", canonical)),
        ] {
            out = out.replace(&left, &right);
        }
        changed = true;
    }

    // Rewrite `<x14:color>` to `<x14:fillColor>` strictly inside
    // `<x14:dataBar>...</x14:dataBar>` blocks. The plain `x14:color` tag
    // only appears in that context in worksheet XML (sparklines use
    // `colorSeries` / `colorAxis`, never bare `color`); scoping to
    // the open/close pair keeps us safe if a future producer emits an
    // `<x14:color>` somewhere else.
    if rewrite_x14_databar_color(&mut out) {
        changed = true;
    }

    if unfold_mc_alternate_content(&mut out) {
        changed = true;
    }

    if changed {
        Some(out.into_bytes())
    } else {
        None
    }
}

/// Rename `<x14:color ...>` and `</x14:color>` to `x14:fillColor` inside
/// every `<x14:dataBar>...</x14:dataBar>` span in `s`. Returns `true` if at
/// least one replacement was applied. Operates on the string in-place by
/// rebuilding it segment-by-segment (cheap: dataBar blocks are tiny).
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
            // Unterminated block: copy the rest verbatim.
            None => {
                out.push_str(after_open);
                rest = "";
                break;
            }
        };
        let block = &after_open[..close_rel];
        // Anchor on the next byte so `<x14:colorScale>` and similar tags
        // are not rewritten.
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
                // Match only `<x14:color` followed by a tag terminator;
                // not `<x14:colorScale` etc.
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
            // Copy one UTF-8 char to preserve non-ASCII text outside tags.
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

/// Unfold every `<mc:AlternateContent>...</mc:AlternateContent>` block in
/// `s` to its first `<mc:Choice>` content (or `<mc:Fallback>` content when
/// no Choice is present). Returns `true` if at least one block was
/// rewritten. Used for drawing parts so chartEx graphics (which Excel
/// always wraps in `mc:AlternateContent` for old-Excel fallback) become
/// plain `<xdr:graphicFrame>` children that ooxmlsdk's typed parser can
/// route into `two_cell_anchor_choice`.
///
/// We deliberately pick the first Choice unconditionally rather than
/// inspecting `Requires="..."`. The set of namespaces this codebase
/// renders is a superset of what Excel knows about for fallback purposes,
/// so the Choice payload is always the richer one. If we ever support a
/// version where Fallback is strictly newer (rare), this can flip.
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
        // Find the matching `</mc:AlternateContent>` (no nesting in real
        // OOXML, but we still account for empty `<mc:AlternateContent/>`).
        // First: is it a self-closing empty block?
        let tag_end = match after_open.find('>') {
            Some(i) => i,
            None => {
                out.push_str(after_open);
                rest = "";
                break;
            }
        };
        if after_open.as_bytes()[tag_end.saturating_sub(1)] == b'/' {
            // Empty `<mc:AlternateContent/>`: drop it.
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
        // Try `<mc:Choice ...>...</mc:Choice>` first.
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

/// Return the inner content of the first `<TAG ...>...</TAG>` block in `s`,
/// or `None` if no such block is found. `tag` is matched literally (it
/// must already include the namespace prefix).
fn extract_mc_container<'a>(s: &'a str, tag: &str) -> Option<&'a str> {
    let open_token = format!("<{}", tag);
    let close_token = format!("</{}>", tag);
    let open_idx = s.find(&open_token)?;
    let after_open = &s[open_idx..];
    // Self-closing block carries no content.
    let tag_end = after_open.find('>')?;
    if after_open.as_bytes()[tag_end.saturating_sub(1)] == b'/' {
        return Some("");
    }
    let close_rel = after_open.find(&close_token)?;
    Some(&after_open[tag_end + 1..close_rel])
}

/// Yield `(prefix, uri)` pairs for every `xmlns:PFX="URI"` (or single-quoted)
/// declaration found in the input. Lightweight scanner; does not try to be a
/// full XML parser but is sufficient for machine-emitted OOXML parts.
fn scan_xmlns_prefixes(s: &str) -> Vec<(&str, &str)> {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while let Some(rel) = s[i..].find("xmlns:") {
        let start = i + rel + "xmlns:".len();
        // Read NCName.
        let mut end = start;
        while end < bytes.len() {
            let c = bytes[end];
            if c == b'=' || c.is_ascii_whitespace() {
                break;
            }
            end += 1;
        }
        let prefix = &s[start..end];
        // Skip whitespace and `=`.
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
        // Hypothetical: if `<x14:color>` ever showed up outside dataBar we
        // must not touch it. (Not a real OOXML shape today, but the guard
        // keeps the rewrite scoped.)
        let xml = br#"<root><x14:color rgb="FFAA0000"/></root>"#;
        assert!(rewrite_xml(xml).is_none());
    }

    #[test]
    fn does_not_match_color_scale_substring() {
        let xml = br#"<x14:dataBar><x14:colorScale/></x14:dataBar>"#;
        // No standalone `<x14:color>`, only `<x14:colorScale>`; leave it alone.
        assert!(rewrite_xml(xml).is_none());
    }

    #[test]
    fn ignores_unknown_namespace() {
        let xml = br#"<foo:bar xmlns:foo="http://example.com/unknown"/>"#;
        assert!(rewrite_xml(xml).is_none());
    }
}
