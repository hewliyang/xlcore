#!/usr/bin/env bash
# Build small, public chart regression fixtures. These intentionally replace
# private real-world workbooks with minimal OOXML repros.
set -euo pipefail
OUT_DIR=${1:-$(dirname "$0")}
python3 - "$OUT_DIR" <<'PY'
from pathlib import Path
import sys, xlsxwriter
out=Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)

def book(name):
    p=out/name
    wb=xlsxwriter.Workbook(str(p))
    ws=wb.add_worksheet('Sheet1')
    return p, wb, ws

# 1) Waterfall-like percent stacked columns with an invisible spacer point.
p,wb,ws=book('chart-waterfall-nofill-stacked.xlsx')
ws.write_row('A1',['Step','Base','Increase','Decrease'])
rows=[['Start',0,40,0],['New',40,25,0],['Churn',50,0,15],['End',0,50,0]]
for r,row in enumerate(rows,1): ws.write_row(r,0,row)
ch=wb.add_chart({'type':'column','subtype':'percent_stacked'})
for col,name,color in [(1,'Base','#FFFFFF'),(2,'Increase','#4F81BD'),(3,'Decrease','#C0504D')]:
    opts={'name':name,'categories':'=Sheet1!$A$2:$A$5','values':f'=Sheet1!${chr(65+col)}$2:${chr(65+col)}$5'}
    if name=='Base': opts['fill']={'none':True}; opts['border']={'none':True}
    else: opts['fill']={'color':color}
    ch.add_series(opts)
ch.set_title({'name':'Waterfall via no-fill stack'}); ch.set_legend({'position':'bottom'}); ch.set_size({'width':520,'height':320})
ws.insert_chart('F2',ch); wb.close(); print(p)

# 2) Stacked columns with same theme accent plus distinct tint/shade colors.
p,wb,ws=book('chart-stacked-color-modifiers.xlsx')
ws.write_row('A1',['Quarter','Services','Manpower'])
for r,row in enumerate([['Q1',35,20],['Q2',45,24],['Q3',52,29],['Q4',60,33]],1): ws.write_row(r,0,row)
ch=wb.add_chart({'type':'column','subtype':'stacked'})
for col,name,color in [(1,'Services','#5B9BD5'),(2,'Manpower','#A9D18E')]:
    ch.add_series({'name':name,'categories':'=Sheet1!$A$2:$A$5','values':f'=Sheet1!${chr(65+col)}$2:${chr(65+col)}$5','fill':{'color':color}})
ch.set_title({'name':'Stacked columns: distinct series colors'}); ch.set_legend({'position':'bottom'}); ch.set_size({'width':520,'height':320})
ws.insert_chart('F2',ch); wb.close(); print(p)

# 3) Column + line combo with the line on a secondary y axis.
p,wb,ws=book('chart-combo-secondary-axis.xlsx')
ws.write_row('A1',['Month','Guards','Avg salary'])
for r,row in enumerate([['Jan',80,2100],['Feb',86,2150],['Mar',91,2230],['Apr',96,2280],['May',103,2350]],1): ws.write_row(r,0,row)
col=wb.add_chart({'type':'column'}); line=wb.add_chart({'type':'line'})
col.add_series({'name':'Guards','categories':'=Sheet1!$A$2:$A$6','values':'=Sheet1!$B$2:$B$6','fill':{'color':'#4472C4'}})
line.add_series({'name':'Avg salary','categories':'=Sheet1!$A$2:$A$6','values':'=Sheet1!$C$2:$C$6','y2_axis':True,'line':{'color':'#ED7D31','width':2.25},'marker':{'type':'circle','size':5}})
col.combine(line); col.set_title({'name':'Combo with secondary axis'}); col.set_y_axis({'name':'Guards'}); col.set_y2_axis({'name':'Salary'}); col.set_legend({'position':'bottom'}); col.set_size({'width':560,'height':330})
ws.insert_chart('F2',col); wb.close(); print(p)

# 4) Two line series on primary/secondary y axes, both non-zero domains.
p,wb,ws=book('chart-dual-axis-lines.xlsx')
ws.write_row('A1',['Quarter','SG&A','SG&A %'])
for r,row in enumerate([['Q1',12.4,.182],['Q2',13.1,.176],['Q3',13.8,.169],['Q4',14.2,.161]],1): ws.write_row(r,0,row)
line1=wb.add_chart({'type':'line'}); line2=wb.add_chart({'type':'line'})
line1.add_series({'name':'SG&A','categories':'=Sheet1!$A$2:$A$5','values':'=Sheet1!$B$2:$B$5','line':{'color':'#70AD47','width':2.25},'marker':{'type':'circle','size':5}})
line2.add_series({'name':'SG&A %','categories':'=Sheet1!$A$2:$A$5','values':'=Sheet1!$C$2:$C$5','y2_axis':True,'line':{'color':'#FFC000','width':2.25},'marker':{'type':'diamond','size':5}})
line1.combine(line2); line1.set_title({'name':'Dual-axis lines'}); line1.set_y_axis({'name':'SG&A'}); line1.set_y2_axis({'name':'SG&A %','num_format':'0.0%'}); line1.set_legend({'position':'bottom'}); line1.set_size({'width':560,'height':330})
ws.insert_chart('F2',line1); wb.close(); print(p)
PY
