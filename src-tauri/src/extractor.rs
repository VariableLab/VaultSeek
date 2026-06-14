use std::path::Path;
use zip::ZipArchive;
use calamine::{Reader, Xlsx, open_workbook};

pub fn extract_text(path: &Path) -> String {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    match ext {
        "pdf" => {
            std::panic::catch_unwind(|| {
                pdf_extract::extract_text(path).unwrap_or_else(|e| {
                    eprintln!("PDF extract failed for {:?}: {:?}", path, e);
                    String::new()
                })
            }).unwrap_or_else(|_| {
                eprintln!("PDF extract panicked for {:?}", path);
                String::new()
            })
        },
        "docx" => {
            std::panic::catch_unwind(|| {
                let file = std::fs::File::open(path).ok()?;
                let mut archive = ZipArchive::new(file).ok()?;
                let mut document_xml = archive.by_name("word/document.xml").ok()?;
                let mut xml_content = String::new();
                std::io::Read::read_to_string(&mut document_xml, &mut xml_content).ok()?;
                
                let mut text = String::new();
                let mut in_tag = false;
                let mut last_was_tag_end = false;
                for c in xml_content.chars() {
                    if c == '<' {
                        in_tag = true;
                    } else if c == '>' {
                        in_tag = false;
                        last_was_tag_end = true;
                    } else if !in_tag {
                        if last_was_tag_end && !text.is_empty() && !text.ends_with(' ') {
                             // 简单猜测段落间隔
                        }
                        text.push(c);
                        last_was_tag_end = false;
                    }
                }
                Some(text)
            }).unwrap_or_default().unwrap_or_default()
        },
        "xlsx" => {
            std::panic::catch_unwind(|| {
                let mut workbook: Xlsx<_> = match open_workbook(path) {
                    Ok(wb) => wb,
                    Err(e) => {
                        eprintln!("XLSX open failed for {:?}: {:?}", path, e);
                        return String::new();
                    }
                };
                let mut markdown = String::new();
                if let Some(Ok(r)) = workbook.worksheet_range_at(0) {
                    for row in r.rows() {
                        markdown.push_str("| ");
                        for cell in row { markdown.push_str(&format!("{:?} | ", cell)); }
                        markdown.push_str("\n");
                    }
                }
                markdown
            }).unwrap_or_default()
        },
        _ => std::fs::read_to_string(path).unwrap_or_default(),
    }
}

pub fn chunk_text(text: &str, max_size: usize, overlap: usize) -> Vec<String> {
    if text.trim().is_empty() { return Vec::new(); }
    
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    
    let lines: Vec<&str> = text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    
    for line in lines {
        let line_len = line.chars().count();
        let current_len = current_chunk.chars().count();
        
        if current_len > 0 && current_len + line_len > max_size {
            chunks.push(current_chunk.trim().to_string());
            
            let chars: Vec<char> = current_chunk.chars().collect();
            let overlap_start = chars.len().saturating_sub(overlap);
            current_chunk = chars[overlap_start..].iter().collect();
            if !current_chunk.is_empty() && !current_chunk.ends_with('\n') {
                current_chunk.push('\n');
            }
        }
        
        if line_len > max_size {
            let mut start = 0;
            let line_chars: Vec<char> = line.chars().collect();
            while start < line_chars.len() {
                let remaining_space = max_size.saturating_sub(current_chunk.chars().count());
                if remaining_space == 0 {
                    chunks.push(current_chunk.trim().to_string());
                    let chars: Vec<char> = current_chunk.chars().collect();
                    let overlap_start = chars.len().saturating_sub(overlap);
                    current_chunk = chars[overlap_start..].iter().collect();
                    continue;
                }
                let end = (start + remaining_space).min(line_chars.len());
                let slice: String = line_chars[start..end].iter().collect();
                current_chunk.push_str(&slice);
                start = end;
            }
            current_chunk.push('\n');
        } else {
            current_chunk.push_str(line);
            current_chunk.push('\n');
        }
    }
    
    if !current_chunk.trim().is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }
    
    chunks
}
