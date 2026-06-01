use crate::model::AddPaperParams;

/// 通过 DOI 从 CrossRef API 抓取元数据
pub fn fetch_by_doi(doi: &str) -> Result<AddPaperParams, String> {
    let url = format!("https://api.crossref.org/works/{}", doi);
    let client = reqwest::blocking::Client::builder()
        .user_agent("Refstore/0.1 (mailto:refstore@example.com)")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let resp: serde_json::Value = client
        .get(&url)
        .send()
        .map_err(|e| format!("Request failed: {}", e))?
        .json()
        .map_err(|e| format!("Parse error: {}", e))?;

    let msg = resp
        .get("message")
        .ok_or("No 'message' in CrossRef response")?;

    let title = msg
        .get("title")
        .and_then(|t| t.as_array())
        .and_then(|a| a.first())
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();

    let authors: Vec<String> = msg
        .get("author")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    let family = a.get("family").and_then(|f| f.as_str()).unwrap_or("");
                    let given = a.get("given").and_then(|g| g.as_str()).unwrap_or("");
                    if family.is_empty() {
                        None
                    } else if given.is_empty() {
                        Some(family.to_string())
                    } else {
                        Some(format!("{} {}", given, family))
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    let abstract_text = msg
        .get("abstract")
        .and_then(|a| a.as_str())
        .map(|s| s.to_string());

    let venue = msg
        .get("container-title")
        .and_then(|t| t.as_array())
        .and_then(|a| a.first())
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());

    let publish_date = msg
        .get("published-print")
        .or_else(|| msg.get("published-online"))
        .or_else(|| msg.get("created"))
        .and_then(|d| d.get("date-parts"))
        .and_then(|dp| dp.as_array())
        .and_then(|a| a.first())
        .and_then(|p| p.as_array())
        .and_then(|parts| {
            let year = parts.first().and_then(|y| y.as_i64())?;
            let month = parts.get(1).and_then(|m| m.as_i64()).unwrap_or(1);
            let day = parts.get(2).and_then(|d| d.as_i64()).unwrap_or(1);
            Some(format!("{:04}-{:02}-{:02}", year, month, day))
        });

    let source_url = msg
        .get("URL")
        .and_then(|u| u.as_str())
        .map(|s| s.to_string());

    Ok(AddPaperParams {
        title,
        authors: if authors.is_empty() { None } else { Some(authors) },
        abstract_text,
        source_url,
        doi: Some(doi.to_string()),
        arxiv_id: None,
        pdf_path: None,
        publish_date: publish_date.and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok()),
        venue,
        force: false,
    })
}

/// 通过 arXiv ID 从 arXiv API 抓取元数据（含自动重试）
pub fn fetch_by_arxiv(arxiv_id: &str) -> Result<AddPaperParams, String> {
    let id = super::db::normalize_arxiv_id_pub(arxiv_id);
    let url = format!("https://export.arxiv.org/api/query?id_list={}", id);

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    // Retry up to 3 times with increasing delay (arXiv API rate limits)
    let mut last_err = String::new();
    for attempt in 0..3 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_secs(5 * attempt as u64));
        }
        match client.get(&url).send() {
            Ok(resp) => {
                let body = resp.text().map_err(|e| format!("Read error: {}", e))?;
                // Check for rate limit or empty response
                if body.contains("Rate exceeded") || body.trim().is_empty() {
                    last_err = "arXiv API rate limit exceeded, retrying...".to_string();
                    continue;
                }
                return parse_arxiv_xml(&body, &id);
            }
            Err(e) => {
                last_err = format!("Request failed: {}", e);
                continue;
            }
        }
    }
    Err(format!("arXiv fetch failed after 3 attempts: {}", last_err))
}

/// 简易 XML 解析 arXiv Atom 响应
fn parse_arxiv_xml(xml: &str, arxiv_id: &str) -> Result<AddPaperParams, String> {
    // Extract the first <entry> block to avoid picking up feed-level metadata
    let entry = extract_xml_tag(xml, "entry").unwrap_or_else(|| xml.to_string());

    let title = extract_xml_tag(&entry, "title")
        .filter(|t| !t.contains("arXiv Query") && !t.contains("ArXiv Query"))
        .unwrap_or_default()
        .trim()
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let summary = extract_xml_tag(&entry, "summary")
        .map(|s| s.trim().replace('\n', " ").split_whitespace().collect::<Vec<_>>().join(" "));

    let authors: Vec<String> = extract_all_xml_tags(&entry, "name");

    let published = extract_xml_tag(&entry, "published")
        .and_then(|d| d.trim().chars().take(10).collect::<String>().into())
        .and_then(|d: String| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());

    let source_url = format!("https://arxiv.org/abs/{}", arxiv_id);

    if title.is_empty() {
        return Err("No entry found in arXiv response".to_string());
    }

    Ok(AddPaperParams {
        title,
        authors: if authors.is_empty() { None } else { Some(authors) },
        abstract_text: summary,
        source_url: Some(source_url),
        doi: None,
        arxiv_id: Some(arxiv_id.to_string()),
        pdf_path: None,
        publish_date: published,
        venue: None,
        force: false,
    })
}

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)?;
    let content_start = start + open.len();
    let end = xml[content_start..].find(&close)?;
    Some(xml[content_start..content_start + end].to_string())
}

fn extract_all_xml_tags(xml: &str, tag: &str) -> Vec<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let mut results = Vec::new();
    let mut search_from = 0;

    while let Some(start) = xml[search_from..].find(&open) {
        let content_start = search_from + start + open.len();
        if let Some(end) = xml[content_start..].find(&close) {
            results.push(xml[content_start..content_start + end].trim().to_string());
            search_from = content_start + end + close.len();
        } else {
            break;
        }
    }

    results
}
