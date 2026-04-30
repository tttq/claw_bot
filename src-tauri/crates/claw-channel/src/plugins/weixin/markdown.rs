// Claw Desktop - Markdown转换 - Markdown转微信格式
use regex::Regex;

pub fn normalize_markdown_for_weixin(content: &str) -> String {
    let mut result = content.to_string();

    let h1_re = Regex::new(r"(?m)^# (.+)$").expect("Invalid h1 regex");
    result = h1_re.replace_all(&result, "【$1】").to_string();

    let h2_re = Regex::new(r"(?m)^## (.+)$").expect("Invalid h2 regex");
    result = h2_re.replace_all(&result, "**$1**").to_string();

    let h3_re = Regex::new(r"(?m)^###+\s*(.+)$").expect("Invalid h3 regex");
    result = h3_re.replace_all(&result, "**$1**").to_string();

    let link_re = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").expect("Invalid link regex");
    result = link_re.replace_all(&result, "$1 ($2)").to_string();

    result = convert_tables_to_kv(&result);

    let multi_newline_re = Regex::new(r"\n{3,}").expect("Invalid multi-newline regex");
    result = multi_newline_re.replace_all(&result, "\n\n").to_string();

    result
}

fn convert_tables_to_kv(content: &str) -> String {
    let table_re = Regex::new(r"(?s)(\|.+\|\n\|[-| :]+\|\n((?:\|.+\|\n?)+))").unwrap();
    table_re
        .replace_all(&content, |caps: &regex::Captures| {
            let table_text = &caps[1];
            let mut lines = table_text.lines();
            let header_line = lines.next().unwrap_or("");
            let headers: Vec<&str> = header_line
                .split('|')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            let mut result = String::new();
            for line in lines.skip(0) {
                let cells: Vec<&str> = line
                    .split('|')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty() && !s.starts_with('-'))
                    .collect();
                if cells.len() >= 2 && cells.len() == headers.len() {
                    for (i, cell) in cells.iter().enumerate() {
                        if i < headers.len() {
                            result.push_str(&format!("- {}: {}\n", headers[i], cell));
                        }
                    }
                }
            }
            result
        })
        .to_string()
}

pub fn split_text_for_weixin(content: &str, max_length: usize, compact: bool) -> Vec<String> {
    if compact && content.len() <= max_length {
        return vec![content.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();

    let mut in_code_block = false;
    for line in content.lines() {
        if line.starts_with("```") {
            in_code_block = !in_code_block;
        }

        if !in_code_block && !current.is_empty() && current.len() + line.len() + 1 > max_length {
            chunks.push(current.trim().to_string());
            current.clear();
        }

        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);
    }

    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }

    if chunks.is_empty() {
        chunks.push(content.to_string());
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_h1() {
        let input = "# Hello World";
        let output = normalize_markdown_for_weixin(input);
        assert!(output.contains("【Hello World】"));
    }

    #[test]
    fn test_normalize_link() {
        let input = "[click here](https://example.com)";
        let output = normalize_markdown_for_weixin(input);
        assert!(output.contains("click here (https://example.com)"));
    }

    #[test]
    fn test_split_text() {
        let long_text = "Line 1\n\nLine 2\n\nLine 3";
        let chunks = split_text_for_weixin(long_text, 10, false);
        assert!(!chunks.is_empty());
    }
}
