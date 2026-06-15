using DocumentFormat.OpenXml;
using DocumentFormat.OpenXml.Packaging;
using DocumentFormat.OpenXml.Spreadsheet;

var path = "dotnet.xlsx";
using (var doc = SpreadsheetDocument.Create(path, SpreadsheetDocumentType.Workbook))
{
    var wbPart = doc.AddWorkbookPart();
    wbPart.Workbook = new Workbook();

    var wsPart = wbPart.AddNewPart<WorksheetPart>();
    wsPart.Worksheet = new Worksheet(new SheetData());

    var stylesPart = wbPart.AddNewPart<WorkbookStylesPart>();
    stylesPart.Stylesheet = new Stylesheet();
    stylesPart.Stylesheet.Save();

    var sheets = wbPart.Workbook.AppendChild(new Sheets());
    sheets.Append(new Sheet
    {
        Id = wbPart.GetIdOfPart(wsPart),
        SheetId = 1,
        Name = "Sheet1",
    });
    wbPart.Workbook.Save();
}
System.Console.Error.WriteLine("wrote dotnet.xlsx");
