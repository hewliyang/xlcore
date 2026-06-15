using DocumentFormat.OpenXml;
using DocumentFormat.OpenXml.Packaging;
using DocumentFormat.OpenXml.Spreadsheet;

using (var doc = SpreadsheetDocument.Create("dotnet.xlsx", SpreadsheetDocumentType.Workbook))
{
    var wbPart = doc.AddWorkbookPart();
    wbPart.Workbook = new Workbook();
    wbPart.Workbook.Save();
}
System.Console.Error.WriteLine("wrote dotnet.xlsx");
