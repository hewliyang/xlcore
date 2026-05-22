use std::io::{Cursor, Read, Write};

use crate::error::{FixedAttribute, LoadReport, SchemaErrorKind, XlsxLoadError};

pub(crate) mod fixer;
pub(crate) mod fixers;

use fixer::{Fixer, FixerError};

pub(crate) fn precompile_xlsx(
    bytes: Vec<u8>,
    report: &mut LoadReport,
) -> Result<Vec<u8>, XlsxLoadError> {
    let pipeline = default_pipeline();
    run_pipeline(bytes, &pipeline, report)
}

pub(crate) fn default_pipeline() -> Vec<Box<dyn Fixer>> {

    vec![
        Box::new(fixers::mc_alternate_content::AlternateContentUnfolder),
        Box::new(fixers::prefix_canonicalize::PrefixCanonicalizer),
        Box::new(fixers::cx_axis_id::CxAxisIdShape),
        Box::new(fixers::databar_color::DataBarColorRename),
        Box::new(fixers::attr_sanitize::AttributeTypeSanitizer),
    ]
}

fn run_pipeline(
    bytes: Vec<u8>,
    pipeline: &[Box<dyn Fixer>],
    report: &mut LoadReport,
) -> Result<Vec<u8>, XlsxLoadError> {

    let mut entries = read_zip(&bytes)?;

    let mut any_changed = false;
    for fixer in pipeline {
        for entry in entries.iter_mut() {
            let ZipEntry::File { name, data, .. } = entry else {
                continue;
            };
            if !fixer.applies_to(name) {
                continue;
            }
            match fixer.rewrite(data, name, report) {
                Ok(None) => {}
                Ok(Some(new_data)) => {
                    *data = new_data;
                    any_changed = true;
                }
                Err(FixerError::Xml { part, source }) => {
                    // Record both as a warning (human-readable context) and
                    // as a structured `fixes` entry so callers using
                    // `LoadReport::is_clean` / CLI `--strict` can detect that
                    // a fixer bailed on malformed XML.
                    let message = format!(
                        "precompile: fixer {} skipped {part}: {source}",
                        fixer.name()
                    );
                    report.warnings.push(message);
                    report.fixes.push(FixedAttribute {
                        part: part.clone(),
                        ty: Some(fixer.name().to_string()),
                        field: None,
                        value: Some("xml-parse-error".to_string()),
                        occurrences: 1,
                        kind: SchemaErrorKind::Other,
                    });
                }
                Err(FixerError::Io { part, source }) => {
                    return Err(XlsxLoadError::Other(format!(
                        "precompile: fixer {} write failed on {part}: {source}",
                        fixer.name()
                    )));
                }
            }
        }
    }

    if !any_changed {
        return Ok(bytes);
    }
    write_zip(&entries)
}

enum ZipEntry {
    File { name: String, data: Vec<u8> },
    Dir { name: String },
}

fn read_zip(bytes: &[u8]) -> Result<Vec<ZipEntry>, XlsxLoadError> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))?;
    let mut out = Vec::with_capacity(zip.len());
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let name = entry.name().to_string();
        if entry.is_dir() {
            out.push(ZipEntry::Dir { name });
            continue;
        }
        let mut data = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut data)?;
        out.push(ZipEntry::File { name, data });
    }
    Ok(out)
}

fn write_zip(entries: &[ZipEntry]) -> Result<Vec<u8>, XlsxLoadError> {
    let mut out = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(Cursor::new(&mut out));
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for entry in entries {
            match entry {
                ZipEntry::Dir { name } => {
                    writer.add_directory(name.clone(), opts)?;
                }
                ZipEntry::File { name, data } => {
                    writer.start_file(name.clone(), opts)?;
                    writer
                        .write_all(data)
                        .map_err(|e| XlsxLoadError::Other(format!("zip write: {e}")))?;
                }
            }
        }
        writer
            .finish()
            .map_err(|e| XlsxLoadError::Other(format!("zip finish: {e}")))?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn passthrough_when_no_fixer_matches() {
        let bytes = build_minimal_zip(&[("foo.txt", b"hello" as &[u8])]);
        let mut report = LoadReport::default();
        let out = precompile_xlsx(bytes.clone(), &mut report).expect("ok");
        assert_eq!(out, bytes, "no fixer touched the archive");
        assert!(report.fixes.is_empty());
    }

    #[test]
    fn rewrites_only_affected_parts() {
        let xml = br#"<cx:s><cx:axisId val="7"/></cx:s>"#;
        let bytes = build_minimal_zip(&[
            ("xl/charts/chart1.xml", xml.as_slice()),
            ("other.bin", b"unchanged"),
        ]);
        let mut report = LoadReport::default();
        let out = precompile_xlsx(bytes, &mut report).expect("ok");
        let entries = read_zip(&out).expect("readable");

        let chart = entries
            .iter()
            .find_map(|e| match e {
                ZipEntry::File { name, data } if name == "xl/charts/chart1.xml" => Some(data),
                _ => None,
            })
            .expect("chart entry");
        let s = std::str::from_utf8(chart).unwrap();
        assert!(s.contains("<cx:axisId>7</cx:axisId>"), "{s}");
        assert_eq!(report.fixes.len(), 1);
        assert_eq!(report.fixes[0].part, "xl/charts/chart1.xml");
    }

    fn build_minimal_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut w = zip::ZipWriter::new(Cursor::new(&mut out));
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            for (name, data) in files {
                w.start_file((*name).to_string(), opts).unwrap();
                w.write_all(data).unwrap();
            }
            w.finish().unwrap();
        }
        out
    }
}
