//! Markdown chunking utilities

/// A single chunk of markdown text
#[derive(Debug, Clone)]
pub struct Chunk {
    pub index: usize,
    pub content: String,
    pub heading: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
}

/// Chunk markdown text by headings
pub fn chunk_markdown(text: &str, max_chars: usize) -> Vec<Chunk> {
    if text.trim().is_empty() {
        return Vec::new();
    }

    let sections = split_on_headings(text);
    let mut chunks = Vec::new();

    for (heading, body, start_line) in sections {
        let full = if let Some(ref h) = heading {
            format!("{}\n{}", h, body)
        } else {
            body.clone()
        };

        if full.len() <= max_chars {
            chunks.push(Chunk {
                index: chunks.len(),
                content: full.trim().to_string(),
                heading: heading.clone(),
                start_line,
                end_line: start_line + full.lines().count(),
            });
        } else {
            // Split on paragraphs
            for (para, para_start) in split_on_paragraphs(&body, start_line) {
                if para.len() > max_chars {
                    // Split on lines
                    for (line_chunk, line_start) in split_on_lines(&para, para_start, max_chars) {
                        chunks.push(Chunk {
                            index: chunks.len(),
                            content: line_chunk.trim().to_string(),
                            heading: heading.clone(),
                            start_line: line_start,
                            end_line: line_start + line_chunk.lines().count(),
                        });
                    }
                } else {
                    chunks.push(Chunk {
                        index: chunks.len(),
                        content: para.trim().to_string(),
                        heading: heading.clone(),
                        start_line: para_start,
                        end_line: para_start + para.lines().count(),
                    });
                }
            }
        }
    }

    // Re-index
    for (i, chunk) in chunks.iter_mut().enumerate() {
        chunk.index = i;
    }

    chunks
}

fn split_on_headings(text: &str) -> Vec<(Option<String>, String, usize)> {
    let mut sections = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_body = String::new();
    let mut start_line = 1;
    let mut current_line = 1;

    for line in text.lines() {
        if line.starts_with("# ") || line.starts_with("## ") || line.starts_with("### ") {
            if !current_body.trim().is_empty() || current_heading.is_some() {
                sections.push((current_heading.take(), std::mem::take(&mut current_body), start_line));
            }
            current_heading = Some(line.to_string());
            start_line = current_line;
        } else {
            if current_body.is_empty() && current_heading.is_none() {
                start_line = current_line;
            }
            current_body.push_str(line);
            current_body.push('\n');
        }
        current_line += 1;
    }

    if !current_body.trim().is_empty() || current_heading.is_some() {
        sections.push((current_heading, current_body, start_line));
    }

    sections
}

fn split_on_paragraphs(text: &str, start_line: usize) -> Vec<(String, usize)> {
    let mut paragraphs = Vec::new();
    let mut current = String::new();
    let mut current_line = start_line;
    let mut para_start = start_line;

    for line in text.lines() {
        if line.trim().is_empty() {
            if !current.trim().is_empty() {
                paragraphs.push((std::mem::take(&mut current), para_start));
            }
            para_start = current_line + 1;
        } else {
            if current.is_empty() {
                para_start = current_line;
            }
            current.push_str(line);
            current.push('\n');
        }
        current_line += 1;
    }

    if !current.trim().is_empty() {
        paragraphs.push((current, para_start));
    }

    paragraphs
}

fn split_on_lines(text: &str, start_line: usize, max_chars: usize) -> Vec<(String, usize)> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_line = start_line;
    let mut chunk_start = start_line;

    for line in text.lines() {
        if current.len() + line.len() > max_chars && !current.is_empty() {
            chunks.push((std::mem::take(&mut current), chunk_start));
            chunk_start = current_line;
        }

        if current.is_empty() {
            chunk_start = current_line;
        }

        current.push_str(line);
        current.push('\n');
        current_line += 1;
    }

    if !current.trim().is_empty() {
        chunks.push((current, chunk_start));
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_by_heading() {
        let text = "# Title\n\nContent here.\n\n## Section\n\nMore content.";
        let chunks = chunk_markdown(text, 1000);

        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].content.starts_with("# Title"));
        assert!(chunks[1].content.starts_with("## Section"));
    }
}
