//! xlcore-io: thin facade over `ooxmlsdk` for xlsx round-trip.
//!
//! For v0 we just re-export the parts of ooxmlsdk we use plus a couple of
//! ergonomic helpers (open/save by path, A1 ref parsing). The bigger plan —
//! narrowing the generated schema (~70% size cut) — comes later.

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek};
use std::path::Path;

pub use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
pub use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as spreadsheetml;
use ooxmlsdk::sdk::{
    FileFormatVersion, MarkupCompatibilityProcessMode, MarkupCompatibilityProcessSettings,
    OpenSettings,
};

mod xmlns_normalize;

/// Open an xlsx file from disk.
pub fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<SpreadsheetDocument> {
    let f = File::open(path.as_ref())?;
    open_reader(BufReader::new(f))
}

/// Open an xlsx document from any seekable byte stream.
pub fn open_reader<R: Read + Seek>(mut reader: R) -> anyhow::Result<SpreadsheetDocument> {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    open_bytes(buf)
}

/// Open an xlsx document from in-memory bytes.
pub fn open_bytes(bytes: Vec<u8>) -> anyhow::Result<SpreadsheetDocument> {
    // Some producers (e.g. Google Sheets) bind the threadedcomments namespaces
    // to non-canonical prefixes such as `x18tc`. ooxmlsdk does literal prefix
    // matching against `xltc`/`xltc2`, so we normalize before parsing.
    let bytes = xmlns_normalize::normalize_xlsx(bytes)?;
    // Enable markup-compatibility processing so `mc:AlternateContent`
    // blocks (used by chartEx drawings: `<mc:Choice Requires="cx1">`
    // wraps the `cx:chart` graphicData with the legacy `c:chart` as
    // `<mc:Fallback>`) collapse to the highest-version Choice that the
    // SDK understands. Target Office2016 — the version that introduced
    // chartEx. Without this, ooxmlsdk silently drops the chartEx
    // graphicData (it sits inside an unknown `<mc:AlternateContent>`
    // node, which isn't a valid child of `<a:graphic>`), and our
    // extractor sees an empty drawing.
    let settings = OpenSettings {
        markup_compatibility_process_settings: MarkupCompatibilityProcessSettings {
            process_mode: MarkupCompatibilityProcessMode::ProcessAllParts,
            target_file_format_version: FileFormatVersion::Office2016,
        },
        ..OpenSettings::default()
    };
    Ok(SpreadsheetDocument::new_with_settings(
        std::io::Cursor::new(bytes),
        settings,
    )?)
}

/// Save an xlsx document to disk.
pub fn save<P: AsRef<Path>>(doc: &mut SpreadsheetDocument, path: P) -> anyhow::Result<()> {
    let f = File::create(path.as_ref())?;
    doc.save(BufWriter::new(f))?;
    Ok(())
}

/// Parse "A1", "AB12" -> (row, col), 1-based.
pub fn parse_a1(r: &str) -> Option<(u32, u32)> {
    let mut col = 0u32;
    let mut row = 0u32;
    let mut in_col = true;
    for ch in r.chars() {
        if in_col && ch.is_ascii_alphabetic() {
            col = col * 26 + (ch.to_ascii_uppercase() as u32 - b'A' as u32 + 1);
        } else if ch.is_ascii_digit() {
            in_col = false;
            row = row * 10 + (ch as u32 - b'0' as u32);
        } else {
            return None;
        }
    }
    if row > 0 && col > 0 {
        Some((row, col))
    } else {
        None
    }
}

/// Convert a column number (1-based) to its label, e.g. 1->"A", 27->"AA".
pub fn col_label(mut n: u32) -> String {
    let mut s = String::new();
    while n > 0 {
        let r = ((n - 1) % 26) as u8;
        s.insert(0, (b'A' + r) as char);
        n = (n - 1) / 26;
    }
    s
}

/// Parse "A1:B3" -> ((r1,c1),(r2,c2)).
pub fn parse_range(s: &str) -> Option<((u32, u32), (u32, u32))> {
    let (a, b) = s.split_once(':')?;
    Some((parse_a1(a)?, parse_a1(b)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a1_roundtrip() {
        assert_eq!(parse_a1("A1"), Some((1, 1)));
        assert_eq!(parse_a1("AA10"), Some((10, 27)));
        assert_eq!(col_label(1), "A");
        assert_eq!(col_label(27), "AA");
        assert_eq!(col_label(703), "AAA");
    }
}
