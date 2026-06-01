/// UI translations for the web admin interface
pub struct Translations {
    // Nav
    pub nav_papers: &'static str,
    pub nav_tags: &'static str,
    pub nav_groups: &'static str,
    pub nav_lang_label: &'static str,
    pub nav_lang_url: &'static str,

    // Search
    pub search_placeholder: &'static str,
    pub search_button: &'static str,

    // Table (list.html)
    pub col_num: &'static str,
    pub col_title: &'static str,
    pub col_authors: &'static str,
    pub col_tags: &'static str,
    pub col_status: &'static str,
    pub col_actions: &'static str,
    pub status_read: &'static str,
    pub status_unread: &'static str,
    pub action_view: &'static str,
    pub action_toggle: &'static str,
    pub action_delete: &'static str,
    pub confirm_delete: &'static str,

    // Pagination
    pub page_prefix: &'static str,
    pub page_sep: &'static str,
    pub page_suffix: &'static str,
    pub prev: &'static str,
    pub next: &'static str,
    pub papers_total_suffix: &'static str,

    // Detail page
    pub link_back: &'static str,
    pub label_authors: &'static str,
    pub label_abstract: &'static str,
    pub label_url: &'static str,
    pub label_doi: &'static str,
    pub label_arxiv: &'static str,
    pub label_venue: &'static str,
    pub label_read: &'static str,
    pub label_tags: &'static str,
    pub yes: &'static str,
    pub no: &'static str,
    pub btn_toggle_read: &'static str,
    pub btn_delete: &'static str,
    pub confirm_delete_short: &'static str,
    pub heading_notes: &'static str,
    pub heading_citations: &'static str,
    pub heading_citation_graph: &'static str,

    // Tags page
    pub heading_tags: &'static str,
    pub link_back_short: &'static str,

    // Groups page
    pub heading_groups: &'static str,
    pub group_papers_suffix: &'static str,
}

impl Translations {
    pub fn en() -> Self {
        Translations {
            nav_papers: "Papers",
            nav_tags: "Tags",
            nav_groups: "Groups",
            nav_lang_label: "中文",
            nav_lang_url: "/lang/zh",

            search_placeholder: "Search papers...",
            search_button: "Search",

            col_num: "#",
            col_title: "Title",
            col_authors: "Authors",
            col_tags: "Tags",
            col_status: "Status",
            col_actions: "Actions",
            status_read: "Read",
            status_unread: "Unread",
            action_view: "View",
            action_toggle: "Toggle",
            action_delete: "Delete",
            confirm_delete: "Delete this paper?",

            page_prefix: "Page",
            page_sep: "of",
            page_suffix: "",
            prev: "Prev",
            next: "Next",
            papers_total_suffix: "papers total",

            link_back: "Back to list",
            label_authors: "Authors:",
            label_abstract: "Abstract:",
            label_url: "URL:",
            label_doi: "DOI:",
            label_arxiv: "arXiv:",
            label_venue: "Venue:",
            label_read: "Read:",
            label_tags: "Tags:",
            yes: "Yes",
            no: "No",
            btn_toggle_read: "Toggle Read",
            btn_delete: "Delete",
            confirm_delete_short: "Delete?",
            heading_notes: "Notes",
            heading_citations: "Citations",
            heading_citation_graph: "Citation Graph",

            heading_tags: "Tags",
            link_back_short: "Back",

            heading_groups: "Groups",
            group_papers_suffix: "papers",
        }
    }

    pub fn zh() -> Self {
        Translations {
            nav_papers: "论文",
            nav_tags: "标签",
            nav_groups: "分组",
            nav_lang_label: "EN",
            nav_lang_url: "/lang/en",

            search_placeholder: "搜索论文...",
            search_button: "搜索",

            col_num: "#",
            col_title: "标题",
            col_authors: "作者",
            col_tags: "标签",
            col_status: "状态",
            col_actions: "操作",
            status_read: "已读",
            status_unread: "未读",
            action_view: "查看",
            action_toggle: "切换",
            action_delete: "删除",
            confirm_delete: "确定删除这篇论文？",

            page_prefix: "第",
            page_sep: "页 / 共",
            page_suffix: "页",
            prev: "上一页",
            next: "下一页",
            papers_total_suffix: "篇论文",

            link_back: "返回列表",
            label_authors: "作者：",
            label_abstract: "摘要：",
            label_url: "链接：",
            label_doi: "DOI：",
            label_arxiv: "arXiv：",
            label_venue: "来源：",
            label_read: "已读：",
            label_tags: "标签：",
            yes: "是",
            no: "否",
            btn_toggle_read: "切换已读",
            btn_delete: "删除",
            confirm_delete_short: "删除？",
            heading_notes: "笔记",
            heading_citations: "引用关系",
            heading_citation_graph: "引用关系图",

            heading_tags: "标签",
            link_back_short: "返回",

            heading_groups: "分组",
            group_papers_suffix: "篇论文",
        }
    }
}

/// Resolve language from cookie header and optional query param override
pub fn resolve_lang(headers: &axum::http::HeaderMap, query_lang: Option<&str>) -> String {
    // Query param takes precedence
    if let Some(l) = query_lang {
        let l = l.trim().to_lowercase();
        if l == "zh" || l == "en" {
            return l;
        }
    }
    // Then cookie
    if let Some(cookie) = headers.get("cookie") {
        if let Ok(val) = cookie.to_str() {
            for pair in val.split(';') {
                let pair = pair.trim();
                if let Some(lang_val) = pair.strip_prefix("lang=") {
                    let lang = lang_val.trim().to_lowercase();
                    if lang == "zh" || lang == "en" {
                        return lang;
                    }
                }
            }
        }
    }
    "en".to_string()
}

/// Get translations for a given language
pub fn translations_for(lang: &str) -> Translations {
    match lang {
        "zh" => Translations::zh(),
        _ => Translations::en(),
    }
}
