// Claw Desktop - 网络工具（WebFetch / WebSearch）
//
// WebFetch: HTTP GET 抓取网页/JSON/API 内容
//   - 自动检测 Content-Type (HTML/XML/JSON/文本/二进制)
//   - HTML 标签自动剥离，保留纯文本
//   - 输出截断保护（默认 50KB 上限）
//   - 30s 超时 + 浏览器 UA 伪装
//
// WebSearch: 多引擎并行搜索（DuckDuckGo / Bing / Google）
//   - 域名白名单/黑名单过滤
//   - 结果去重（URL 级别）
//   - 结构化输出（title/url/snippet）
//   - 搜索建议提示

/// 网页抓取工具 — HTTP GET获取内容，自动检测类型并剥离HTML标签
#[tauri::command]
pub async fn tool_web_fetch(url: String, max_length: Option<u64>) -> Result<serde_json::Value, String> {
    let start = std::time::Instant::now();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .danger_accept_invalid_certs(false)
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Ok(serde_json::json!({"tool":"WebFetch","success":false,"output":format!("无效的 URL: {} (必须以 http:// 或 https:// 开头)",url),"duration_ms":start.elapsed().as_millis() as u64}));
    }

    let response = client.get(&url).send().await.map_err(|e| format!("请求失败: {}", e))?;
    let status = response.status();
    if !status.is_success() {
        return Ok(serde_json::json!({
            "tool": "WebFetch", "success": false,
            "output": format!("HTTP {} {}", status, status.canonical_reason().unwrap_or("Unknown")),
            "status_code": status.as_u16(), "duration_ms": start.elapsed().as_millis() as u64
        }));
    }

    let content_type: String = response.headers()
        .get("content-type").and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let bytes = response.bytes().await.map_err(|e| format!("读取响应体失败: {}", e))?;
    let raw = String::from_utf8_lossy(&bytes);
    let max_len = max_length.unwrap_or(50000) as usize;

    let text = if content_type.contains("html") || content_type.contains("xml") {
        strip_html_tags(&raw).split_whitespace().collect::<Vec<_>>().join(" ")
    } else if content_type.contains("json") || content_type.contains("javascript") || content_type.contains("text") {
        raw.trim().to_string()
    } else {
        format!("[二进制数据: {} 字节, Content-Type: {}]", bytes.len(), content_type)
    };

    let truncated = if text.len() > max_len {
        format!("...(截断 {} 字节)\n{}", text.len(), &text[text.len()-max_len..])
    } else { text };

    Ok(serde_json::json!({
        "tool": "WebFetch", "success": true,
        "output": format!("URL: {}\nStatus: HTTP {}\nContent-Type: {}\nSize: {}\nDuration: {}ms\n\n{}", 
            url, status, content_type, bytes.len(), start.elapsed().as_millis() as u64, truncated),
        "status_code": status.as_u16(),
        "content_type": content_type,
        "size_bytes": bytes.len(),
        "duration_ms": start.elapsed().as_millis() as u64
    }))
}

/// 网络搜索工具 — 多引擎并行搜索，支持域名过滤和结果去重
#[tauri::command]
pub async fn tool_web_search(
    query: String,
    engine: Option<String>,
    num_results: Option<u64>,
    allowed_domains: Option<serde_json::Value>,
    blocked_domains: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let start = std::time::Instant::now();
    let count = num_results.unwrap_or(5).min(10) as usize;
    let engine_id = engine.clone().unwrap_or_else(|| "duckduckgo".to_string()).to_lowercase();

    let allowed_list: Vec<String> = match &allowed_domains {
        Some(serde_json::Value::Array(arr)) => arr.iter().filter_map(|v| v.as_str().map(|s| s.to_lowercase())).collect(),
        Some(serde_json::Value::String(s)) => vec![s.to_lowercase()],
        _ => vec![],
    };
    let blocked_list: Vec<String> = match &blocked_domains {
        Some(serde_json::Value::Array(arr)) => arr.iter().filter_map(|v| v.as_str().map(|s| s.to_lowercase())).collect(),
        Some(serde_json::Value::String(s)) => vec![s.to_lowercase()],
        _ => vec![],
    };

    let engines_to_use: Vec<&str> = if engine_id == "all" || engine_id == "multi" {
        vec!["duckduckgo", "bing"]
    } else {
        vec![engine_id.as_str()]
    };

    let mut all_results: Vec<SearchResult> = Vec::new();
    let mut seen_urls = std::collections::HashSet::new();
    
    for eng in &engines_to_use {
        let results = match *eng {
            "duckduckgo" | "ddg" => search_duckduckgo(&query, count * 2).await,
            "bing" => search_bing(&query, count * 2).await,
            "google" | "g" => search_google(&query, count * 2).await,
            _ => search_duckduckgo(&query, count * 2).await,
        };

        for r in results {
            let domain = extract_domain(&r.url);
            if seen_urls.contains(&r.url) { continue; }
            if !allowed_list.is_empty() && !allowed_list.iter().any(|d| domain.ends_with(d) || d == &domain) { continue; }
            if !blocked_list.is_empty() && (blocked_list.iter().any(|d| domain.ends_with(d) || d == &domain) || is_blocked_url(&r.url, &blocked_list)) { continue; }
            
            seen_urls.insert(r.url.clone());
            all_results.push(r);
        }
    }

    all_results.truncate(count);
    let duration_sec = start.elapsed().as_secs_f64();

    if all_results.is_empty() {
        Ok(serde_json::json!({
            "tool": "WebSearch", "success": true, "query": query,
            "results": [],
            "result_count": 0,
            "engine": engine_id,
            "duration_seconds": duration_sec,
            "output": format!(
                "搜索 '{}' 无结果\n引擎: {}\n耗时: {:.1}s{}{}\n提示: 尝试更具体的关键词或更换搜索引擎",
                query, engine_id, duration_sec,
                if !allowed_list.is_empty() { format!("\n域名过滤(允许): {:?}", allowed_list) } else { String::new() },
                if !blocked_list.is_empty() { format!("\n域名过滤(阻止): {:?}", blocked_list) } else { String::new() }
            )
        }))
    } else {
        let structured_results: Vec<serde_json::Value> = all_results.iter().map(|r| {
            serde_json::json!({ "title": r.title, "url": r.url, "snippet": r.snippet })
        }).collect();
        
        let output_lines: Vec<String> = all_results.iter().enumerate().flat_map(|(i, r)| {
            vec![
                format!("[{}] {}", i+1, r.title),
                format!("    URL: {}", r.url),
                format!("    {}", r.snippet),
            ]
        }).collect();

        Ok(serde_json::json!({
            "tool": "WebSearch", "success": true, "query": query,
            "results": structured_results,
            "result_count": all_results.len(),
            "engine": engine_id,
            "duration_seconds": duration_sec,
            "output": format!(
                "搜索 '{}': {} 条结果 | 引擎: {} | 耗时: {:.1}s\n\n{}",
                query, all_results.len(), engine_id, duration_sec, output_lines.join("\n")
            )
        }))
    }
}

/// 搜索结果条目
struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

/// 从URL中提取域名
fn extract_domain(url: &str) -> String {
    url.replace("https://", "").replace("http://", "")
     .split('/').next().unwrap_or("")
     .split(':').next().unwrap_or("")
     .to_lowercase()
}

/// 检查URL是否被黑名单阻止
fn is_blocked_url(url: &str, blocked: &[String]) -> bool {
    let lower = url.to_lowercase();
    blocked.iter().any(|b| lower.contains(b))
}

/// DuckDuckGo搜索引擎 — 抓取HTML页面并解析搜索结果
async fn search_duckduckgo(query: &str, count: usize) -> Vec<SearchResult> {
    let search_url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build();

    let Ok(client) = client else { return vec![] };
    let Ok(response) = client.get(&search_url).send().await else { return vec![] };
    if !response.status().is_success() { return vec![]; }
    let Ok(html) = response.text().await else { return vec![] };

    let re = regex::Regex::new(r#"class="result__a"[^>]*href="(.*?)"[^>]*>(.*?)</a>.*?class="result__snippet"[^>]*>(.*?)(?:</div>|</td>)"#)
        .unwrap_or_else(|_| regex::Regex::new(r#"<a[^>]*class="[^"]*result__a[^"]*"[^>]*href="(.*?)"[^>]*>(.*?)</a>"#).expect("Failed to create fallback DDG regex"));

    let mut results = Vec::new();
    for cap in re.captures_iter(&html) {
        let title = html_decode(cap.get(2).map(|m| m.as_str()).unwrap_or(""));
        let raw_url = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let url = clean_ddg_url(raw_url);
        let snippet = html_decode(cap.get(3).map(|m| m.as_str()).unwrap_or(""));
        if !title.is_empty() && results.len() < count {
            results.push(SearchResult { title, url, snippet });
        }
    }
    results
}

/// Bing搜索引擎 — 抓取HTML页面并解析搜索结果
async fn search_bing(query: &str, count: usize) -> Vec<SearchResult> {
    let search_url = format!("https://www.bing.com/search?q={}&count={}", urlencoding::encode(query), count);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build();

    let Ok(client) = client else { return vec![] };
    let Ok(response) = client.get(&search_url).send().await else { return vec![] };
    if !response.status().is_success() { return vec![]; }
    let Ok(html) = response.text().await else { return vec![] };

    let re = regex::Regex::new(r#"<li class="b_algo"[^>]*>.*?<h2><a[^>]*href="(.*?)"[^>]*>(.*?)</a></h2>.*?<p[^>]*>(.*?)</p>"#)
        .unwrap_or_else(|_| regex::Regex::new(r#"<h2><a[^>]*href="(.*?)"[^>]*>(.*?)</a></h2>"#).expect("Failed to create fallback Bing regex"));

    let mut results = Vec::new();
    for cap in re.captures_iter(&html) {
        let url = cap.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        let title = strip_html_tags(cap.get(2).map(|m| m.as_str()).unwrap_or(""));
        let snippet = strip_html_tags(cap.get(3).map(|m| m.as_str()).unwrap_or(""));
        if !title.is_empty() && results.len() < count {
            results.push(SearchResult { title, url, snippet });
        }
    }
    results
}

/// Google搜索引擎 — 抓取HTML页面并解析搜索结果
async fn search_google(query: &str, count: usize) -> Vec<SearchResult> {
    let search_url = format!("https://www.google.com/search?q={}&num={}", urlencoding::encode(query), count);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build();

    let Ok(client) = client else { return vec![] };
    let Ok(response) = client.get(&search_url).send().await else { return vec![] };
    if !response.status().is_success() { return vec![]; }
    let Ok(html) = response.text().await else { return vec![] };

    let re = regex::Regex::new(r#"<div[^>]*class="[^"]*g[^"]*"[^>]*>.*?<a href="/url\?q=(.*?)"[^>]*>.*?<h3[^>]*>(.*?)</h3>.*?(?:<span[^>]*class="[^"]*"[^>]*>(.*?)</span>)?"#)
        .unwrap_or_else(|_| regex::Regex::new(r#"<a href="/url\?q=(.*?)"[^>]*data-ved[^>]*>.*?<h3[^>]*>(.*?)</h3>"#).expect("Failed to create fallback Google regex"));

    let mut results = Vec::new();
    for cap in re.captures_iter(&html) {
        let url = cap.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        let title = strip_html_tags(cap.get(2).map(|m| m.as_str()).unwrap_or(""));
        let snippet = cap.get(3).map(|m| strip_html_tags(m.as_str())).unwrap_or_default();
        if !title.is_empty() && url.starts_with("http") && results.len() < count {
            results.push(SearchResult { title, url, snippet });
        }
    }
    results
}

/// 清理DuckDuckGo URL — 补全协议前缀
fn clean_ddg_url(raw: &str) -> String {
    if raw.starts_with("//") { format!("https:{}", raw) }
    else if raw.starts_with("/") { format!("https://duckduckgo.com{}", raw) }
    else { raw.to_string() }
}

/// HTML实体解码 — 替换常见HTML转义字符
fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"")
     .replace("&#39;", "'").replace("&nbsp;", " ").replace("&#x27;", "'")
     .replace("&mdash;", "-").replace("&ndash;", "-").replace("&hellip;", "...")
     .split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 剥离HTML标签 — 保留纯文本内容
fn strip_html_tags(s: &str) -> String {
    let (mut out, mut in_tag) = (String::new(), false);
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    html_decode(&out)
}
