use std::io::{Cursor, Read, Write};

use crate::errors::zip_err;
use crate::Result;
use crate::{ApiError, ApiErrorCode, Workbook};

fn normalize(name: &str) -> String {
    name.trim_start_matches('/').to_string()
}

impl Workbook {
    fn package_bytes(&self) -> Result<Vec<u8>> {
        let bytes = self
            .doc
            .to_package_bytes()
            .map_err(|err| ApiError::new(ApiErrorCode::OoxmlWriteError, err.to_string()))?;
        Ok(bytes)
    }

    pub fn part_names(&self) -> Result<Vec<String>> {
        let bytes = self.package_bytes()?;
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(zip_err)?;
        let mut names = Vec::with_capacity(archive.len());
        for i in 0..archive.len() {
            let file = archive.by_index(i).map_err(zip_err)?;
            if !file.is_dir() {
                names.push(file.name().to_string());
            }
        }
        names.sort();
        Ok(names)
    }

    pub fn get_part_xml(&self, name: &str) -> Result<Option<String>> {
        let name = normalize(name);
        let bytes = self.package_bytes()?;
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(zip_err)?;
        let mut file = match archive.by_name(&name) {
            Ok(file) => file,
            Err(zip::result::ZipError::FileNotFound) => return Ok(None),
            Err(err) => return Err(zip_err(err)),
        };
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(zip_err)?;
        String::from_utf8(buf).map(Some).map_err(|_| {
            ApiError::new(
                ApiErrorCode::Other,
                format!("part '{name}' is not UTF-8 text"),
            )
        })
    }

    pub fn set_part_xml(&mut self, name: &str, xml: &str) -> Result<()> {
        self.write_part(&normalize(name), Some(xml.as_bytes()))
    }

    pub fn remove_part_xml(&mut self, name: &str) -> Result<bool> {
        let name = normalize(name);
        let existed = self.part_names()?.iter().any(|existing| existing == &name);
        if existed {
            self.write_part(&name, None)?;
        }
        Ok(existed)
    }

    fn write_part(&mut self, name: &str, content: Option<&[u8]>) -> Result<()> {
        let bytes = self.package_bytes()?;
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(zip_err)?;
        let mut out = Cursor::new(Vec::new());
        let mut written = false;
        {
            let mut zip = zip::ZipWriter::new(&mut out);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .unix_permissions(0o644);
            for i in 0..archive.len() {
                let mut file = archive.by_index(i).map_err(zip_err)?;
                if file.is_dir() {
                    continue;
                }
                let entry = file.name().to_string();
                if entry == name {
                    if let Some(content) = content {
                        zip.start_file(name, options).map_err(zip_err)?;
                        zip.write_all(content).map_err(zip_err)?;
                    }
                    written = true;
                    continue;
                }
                let mut buf = Vec::new();
                file.read_to_end(&mut buf).map_err(zip_err)?;
                zip.start_file(entry, options).map_err(zip_err)?;
                zip.write_all(&buf).map_err(zip_err)?;
            }
            if !written {
                if let Some(content) = content {
                    zip.start_file(name, options).map_err(zip_err)?;
                    zip.write_all(content).map_err(zip_err)?;
                }
            }
            zip.finish().map_err(zip_err)?;
        }
        let (doc, report) = xlcore_io::open_bytes_with_report(out.into_inner())
            .map_err(crate::errors::load_err_to_api)?;
        self.doc = doc;
        self.report = report;
        self.invalidate_engine();
        Ok(())
    }
}
