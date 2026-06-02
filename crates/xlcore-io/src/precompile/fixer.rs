use std::io::Cursor;

use quick_xml::events::Event;
use quick_xml::{Reader, Writer};

use crate::error::LoadReport;

pub(crate) trait Fixer: Send + Sync {
    fn name(&self) -> &'static str;

    fn applies_to(&self, part: &str) -> bool {
        is_xml_part(part) && !part.contains("/_rels/")
    }

    fn rewrite(
        &self,
        xml: &[u8],
        part: &str,
        report: &mut LoadReport,
    ) -> Result<Option<Vec<u8>>, FixerError>;
}

pub(crate) fn is_xml_part(name: &str) -> bool {
    name.ends_with(".xml") || name.ends_with(".rels")
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum FixerError {
    #[error("xml parse error in {part}: {source}")]
    Xml {
        part: String,
        #[source]
        source: quick_xml::Error,
    },
    #[error("xml write error in {part}: {source}")]
    Io {
        part: String,
        #[source]
        source: std::io::Error,
    },
}

pub(crate) struct Ctx<'a> {
    pub part: &'a str,
    pub report: &'a mut LoadReport,

    stack: Vec<Vec<u8>>,
}

impl Ctx<'_> {
    pub fn in_element(&self, qname: &[u8]) -> bool {
        self.stack.iter().any(|n| n == qname)
    }
}

pub(crate) enum Emit<'a> {
    Keep(Event<'a>),

    Replace { event: Event<'a>, changed: bool },

    Drop,

    Many(Vec<Event<'a>>),
}

pub(crate) fn sax_rewrite<F>(
    xml: &[u8],
    part: &str,
    report: &mut LoadReport,
    mut mapper: F,
) -> Result<Option<Vec<u8>>, FixerError>
where
    F: for<'e> FnMut(&mut Ctx<'_>, Event<'e>) -> Emit<'e>,
{
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);

    let mut out: Vec<u8> = Vec::with_capacity(xml.len());
    let mut writer = Writer::new(Cursor::new(&mut out));
    let mut buf = Vec::new();
    let mut stack: Vec<Vec<u8>> = Vec::with_capacity(16);
    let mut changed = false;

    loop {
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| FixerError::Xml {
                part: part.to_string(),
                source: e,
            })?;

        match &event {
            Event::Start(e) => stack.push(e.name().as_ref().to_vec()),
            Event::End(_) => {
                stack.pop();
            }
            _ => {}
        }
        let _ = &stack;

        let is_eof = matches!(event, Event::Eof);

        let mut ctx = Ctx {
            part,
            report,
            stack: std::mem::take(&mut stack),
        };
        let action = mapper(&mut ctx, event);
        stack = std::mem::take(&mut ctx.stack);

        match action {
            Emit::Keep(ev) => write_event(&mut writer, ev, part)?,
            Emit::Replace { event, changed: c } => {
                if c {
                    changed = true;
                }
                write_event(&mut writer, event, part)?;
            }
            Emit::Drop => {
                changed = true;
            }
            Emit::Many(events) => {
                changed = true;
                for ev in events {
                    write_event(&mut writer, ev, part)?;
                }
            }
        }

        if is_eof {
            break;
        }
        buf.clear();
    }

    drop(writer);
    Ok(if changed { Some(out) } else { None })
}

fn write_event<W: std::io::Write>(
    w: &mut Writer<W>,
    ev: Event<'_>,
    part: &str,
) -> Result<(), FixerError> {
    w.write_event(ev).map_err(|e| FixerError::Io {
        part: part.to_string(),
        source: e,
    })
}
