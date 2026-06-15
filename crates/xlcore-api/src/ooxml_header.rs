use ooxmlsdk::common::{XmlHeaderType, XmlNamespace};

pub(crate) const STANDALONE: XmlHeaderType = XmlHeaderType::Standalone;

const SPREADSHEETML: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const RELATIONSHIPS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const DRAWINGML_MAIN: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const DRAWINGML_CHART: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
const SPREADSHEET_DRAWING: &str =
    "http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing";
const THREADED_COMMENTS: &str =
    "http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments";

pub(crate) fn ns(prefix: &str, uri: &str) -> XmlNamespace {
    let mut raw = Vec::with_capacity(prefix.len() + 1 + uri.len());
    raw.extend_from_slice(prefix.as_bytes());
    raw.push(0);
    raw.extend_from_slice(uri.as_bytes());
    XmlNamespace::Raw(raw.into_boxed_slice())
}

pub(crate) fn spreadsheetml_default() -> Vec<XmlNamespace> {
    vec![ns("", SPREADSHEETML), ns("r", RELATIONSHIPS)]
}

pub(crate) fn spreadsheetml_default_only() -> Vec<XmlNamespace> {
    vec![ns("", SPREADSHEETML)]
}

pub(crate) fn drawing_root() -> Vec<XmlNamespace> {
    vec![
        ns("xdr", SPREADSHEET_DRAWING),
        ns("a", DRAWINGML_MAIN),
        ns("r", RELATIONSHIPS),
        ns("c", DRAWINGML_CHART),
    ]
}

pub(crate) fn chart_space() -> Vec<XmlNamespace> {
    vec![
        ns("c", DRAWINGML_CHART),
        ns("a", DRAWINGML_MAIN),
        ns("r", RELATIONSHIPS),
    ]
}

const CHARTEX: &str = "http://schemas.microsoft.com/office/drawing/2014/chartex";

pub(crate) fn chart_ex_space() -> Vec<XmlNamespace> {
    vec![
        ns("a", DRAWINGML_MAIN),
        ns("r", RELATIONSHIPS),
        ns("cx", CHARTEX),
    ]
}

pub(crate) fn threaded_comments() -> Vec<XmlNamespace> {
    vec![ns("xltc", THREADED_COMMENTS)]
}
