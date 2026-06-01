use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as xspread;

#[derive(Default)]
pub(crate) struct FlatFont<'a> {
    pub bold: Option<&'a xspread::Bold>,
    pub italic: Option<&'a xspread::Italic>,
    pub strike: Option<&'a xspread::Strike>,
    pub underline: Option<&'a xspread::Underline>,
    pub vertical_text_alignment: Option<&'a xspread::VerticalTextAlignment>,
    pub font_size: Option<&'a xspread::FontSize>,
    pub color: Option<&'a xspread::Color>,
    pub font_name: Option<&'a xspread::FontName>,
    pub font_family_numbering: Option<&'a xspread::FontFamilyNumbering>,
    pub font_scheme: Option<&'a xspread::FontScheme>,
}

pub(crate) fn flatten_font(f: &xspread::Font) -> FlatFont<'_> {
    use xspread::FontChoice as C;
    let mut out = FlatFont::default();
    for c in f.font_choice.iter() {
        match c {
            C::Bold(b) => out.bold = Some(b),
            C::Italic(i) => out.italic = Some(i),
            C::Strike(s) => out.strike = Some(s),
            C::Underline(u) => out.underline = Some(u),
            C::VerticalTextAlignment(v) => out.vertical_text_alignment = Some(v),
            C::FontSize(s) => out.font_size = Some(s),
            C::Color(c) => out.color = Some(c),
            C::FontName(n) => out.font_name = Some(n),
            C::FontFamilyNumbering(fm) => out.font_family_numbering = Some(fm),
            C::FontScheme(s) => out.font_scheme = Some(s),
            _ => {}
        }
    }
    out
}

#[derive(Default)]
pub(crate) struct FlatRunProps<'a> {
    pub bold: Option<&'a xspread::Bold>,
    pub italic: Option<&'a xspread::Italic>,
    pub strike: Option<&'a xspread::Strike>,
    pub underline: Option<&'a xspread::Underline>,
    pub vertical_text_alignment: Option<&'a xspread::VerticalTextAlignment>,
    pub font_size: Option<&'a xspread::FontSize>,
    pub color: Option<&'a xspread::Color>,
    pub run_font: Option<&'a xspread::RunFont>,
    pub font_family: Option<&'a xspread::FontFamily>,
    pub font_scheme: Option<&'a xspread::FontScheme>,
}

pub(crate) fn flatten_run_properties(r: &xspread::RunProperties) -> FlatRunProps<'_> {
    use xspread::RunPropertiesChoice as C;
    let mut out = FlatRunProps::default();
    for c in r.run_properties_choice.iter() {
        match c {
            C::Bold(b) => out.bold = Some(b),
            C::Italic(i) => out.italic = Some(i),
            C::Strike(s) => out.strike = Some(s),
            C::Underline(u) => out.underline = Some(u),
            C::VerticalTextAlignment(v) => out.vertical_text_alignment = Some(v),
            C::FontSize(s) => out.font_size = Some(s),
            C::Color(c) => out.color = Some(c),
            C::RunFont(n) => out.run_font = Some(n),
            C::FontFamily(fm) => out.font_family = Some(fm),
            C::FontScheme(s) => out.font_scheme = Some(s),
            _ => {}
        }
    }
    out
}
