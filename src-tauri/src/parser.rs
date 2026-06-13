use docx_rs::read_docx;
use calamine::{Reader, Xlsx, open_workbook};
use std::path::Path;
use std::fs::File;
use std::io::Read;

pub struct Parser;

impl Parser {
    pub fn parse_docx(path: &Path) -> Result<String, String> {
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
        let docx = read_docx(&buffer).map_err(|e| e.to_string())?;
        
        let mut markdown = String::new();
        // 简单提取文本内容
        for p in docx.document.children.iter() {
            markdown.push_str(&format!("{:?}", p));
            markdown.push_str("\n\n");
        }
        Ok(markdown)
    }

    pub fn parse_xlsx(path: &Path) -> Result<String, String> {
        let mut workbook: Xlsx<_> = open_workbook(path).map_err(|e: calamine::XlsxError| e.to_string())?;
        let mut markdown = String::new();
        
        for sheet_name in workbook.sheet_names().to_vec() {
            markdown.push_str(&format!("# Sheet: {}\n\n", sheet_name));
            if let Ok(r) = workbook.worksheet_range(&sheet_name) {
                markdown.push_str("| ");
                // 假设第一行是表头
                if let Some(first_row) = r.rows().next() {
                    for cell in first_row {
                        markdown.push_str(&format!(" {} |", cell));
                    }
                    markdown.push_str("\n| ");
                    for _ in first_row {
                        markdown.push_str(" --- |");
                    }
                    markdown.push_str("\n");
                }
                
                // 处理其余行
                for row in r.rows().skip(1) {
                    markdown.push_str("| ");
                    for cell in row {
                        markdown.push_str(&format!(" {} |", cell));
                    }
                    markdown.push_str("\n");
                }
            }
            markdown.push_str("\n\n");
        }
        Ok(markdown)
    }
}
