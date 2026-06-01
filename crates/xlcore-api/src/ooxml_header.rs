use ooxmlsdk::common::{XmlHeaderType, XmlNamespaceDecl};

pub(crate) const STANDALONE: XmlHeaderType = XmlHeaderType::Standalone;

const SPREADSHEETML: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const RELATIONSHIPS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const DRAWINGML_MAIN: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const DRAWINGML_CHART: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
const SPREADSHEET_DRAWING: &str =
    "http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing";
const THREADED_COMMENTS: &str =
    "http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments";

fn ns(prefix: &str, uri: &str) -> XmlNamespaceDecl {
    XmlNamespaceDecl {
        prefix: prefix.into(),
        uri: uri.into(),
    }
}

pub(crate) fn spreadsheetml_default() -> Vec<XmlNamespaceDecl> {
    vec![ns("", SPREADSHEETML), ns("r", RELATIONSHIPS)]
}

pub(crate) fn spreadsheetml_default_only() -> Vec<XmlNamespaceDecl> {
    vec![ns("", SPREADSHEETML)]
}

pub(crate) fn drawing_root() -> Vec<XmlNamespaceDecl> {
    vec![
        ns("xdr", SPREADSHEET_DRAWING),
        ns("a", DRAWINGML_MAIN),
        ns("r", RELATIONSHIPS),
        ns("c", DRAWINGML_CHART),
    ]
}

pub(crate) fn chart_space() -> Vec<XmlNamespaceDecl> {
    vec![
        ns("c", DRAWINGML_CHART),
        ns("a", DRAWINGML_MAIN),
        ns("r", RELATIONSHIPS),
    ]
}

pub(crate) fn threaded_comments() -> Vec<XmlNamespaceDecl> {
    vec![ns("xltc", THREADED_COMMENTS)]
}
