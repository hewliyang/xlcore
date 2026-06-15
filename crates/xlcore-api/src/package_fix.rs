use std::io::{Cursor, Read, Write};

use crate::errors::zip_err;
use crate::Result;

const RELS_DEFAULT: &str = r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#;
const XML_DEFAULT: &str = r#"<Default Extension="xml" ContentType="application/xml"/>"#;

fn read_content_types(bytes: &[u8]) -> Result<Option<String>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(zip_err)?;
    let mut file = match archive.by_name("[Content_Types].xml") {
        Ok(file) => file,
        Err(_) => return Ok(None),
    };
    let mut content_types = String::new();
    file.read_to_string(&mut content_types).map_err(zip_err)?;
    Ok(Some(content_types))
}

pub(crate) fn ensure_content_type_defaults(bytes: Vec<u8>) -> Result<Vec<u8>> {
    let content_types = match read_content_types(&bytes)? {
        Some(value) => value,
        None => return Ok(bytes),
    };

    let mut prefix = String::new();
    if !content_types.contains("Extension=\"rels\"") {
        prefix.push_str(RELS_DEFAULT);
    }
    if !content_types.contains("Extension=\"xml\"") {
        prefix.push_str(XML_DEFAULT);
    }
    if prefix.is_empty() {
        return Ok(bytes);
    }

    let insert_at = match content_types.find("<Types").and_then(|start| {
        content_types[start..]
            .find('>')
            .map(|offset| start + offset + 1)
    }) {
        Some(pos) => pos,
        None => return Ok(bytes),
    };
    let mut patched = content_types;
    patched.insert_str(insert_at, &prefix);

    let mut out = Cursor::new(Vec::new());
    {
        let mut archive = zip::ZipArchive::new(Cursor::new(&bytes)).map_err(zip_err)?;
        let mut zip = zip::ZipWriter::new(&mut out);
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(zip_err)?;
            let name = file.name().to_string();
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(file.compression())
                .unix_permissions(0o644);
            if file.is_dir() {
                zip.add_directory(name, options).map_err(zip_err)?;
            } else if name == "[Content_Types].xml" {
                zip.start_file(name, options).map_err(zip_err)?;
                zip.write_all(patched.as_bytes()).map_err(zip_err)?;
            } else {
                let mut buf = Vec::new();
                file.read_to_end(&mut buf).map_err(zip_err)?;
                zip.start_file(name, options).map_err(zip_err)?;
                zip.write_all(&buf).map_err(zip_err)?;
            }
        }
        zip.finish().map_err(zip_err)?;
    }
    Ok(out.into_inner())
}
