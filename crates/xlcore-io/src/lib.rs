use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek};
use std::path::Path;

pub use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
pub use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as spreadsheetml;
use ooxmlsdk::sdk::{
    FileFormatVersion, MarkupCompatibilityProcessMode, MarkupCompatibilityProcessSettings,
    OpenSettings,
};

mod error;
mod precompile;

use error::map_sdk_error;
pub use error::{FixedAttribute, LoadReport, SchemaErrorKind, XlsxLoadError};

pub fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<SpreadsheetDocument> {
    let f = File::open(path.as_ref())?;
    open_reader(BufReader::new(f))
}

pub fn open_reader<R: Read + Seek>(mut reader: R) -> anyhow::Result<SpreadsheetDocument> {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    open_bytes(buf)
}

pub fn open_bytes(bytes: Vec<u8>) -> anyhow::Result<SpreadsheetDocument> {
    let (doc, _report) = open_bytes_with_report(bytes)?;
    Ok(doc)
}

pub fn open_bytes_with_report(
    bytes: Vec<u8>,
) -> Result<(SpreadsheetDocument, LoadReport), XlsxLoadError> {
    let mut report = LoadReport::default();

    let bytes = precompile::precompile_xlsx(bytes, &mut report)?;

    let settings = OpenSettings {
        markup_compatibility_process_settings: MarkupCompatibilityProcessSettings {
            process_mode: MarkupCompatibilityProcessMode::ProcessAllParts,
            target_file_format_version: FileFormatVersion::Office2016,
        },
        ..OpenSettings::default()
    };

    let doc = SpreadsheetDocument::new_with_settings(std::io::Cursor::new(&bytes), settings)
        .map_err(map_sdk_error)?;
    Ok((doc, report))
}

pub fn save<P: AsRef<Path>>(doc: &mut SpreadsheetDocument, path: P) -> anyhow::Result<()> {
    let f = File::create(path.as_ref())?;
    doc.save(BufWriter::new(f))?;
    Ok(())
}

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

pub fn col_label(mut n: u32) -> String {
    let mut s = String::new();
    while n > 0 {
        let r = ((n - 1) % 26) as u8;
        s.insert(0, (b'A' + r) as char);
        n = (n - 1) / 26;
    }
    s
}

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
