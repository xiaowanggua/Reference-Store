use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

// ── ID 类型别名 ──

pub type PaperId = String;
pub type TagId = String;
pub type NoteId = String;
pub type GroupId = String;

// ── 核心模型 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub id: PaperId,
    pub title: String,
    pub authors: Vec<String>,
    pub abstract_text: Option<String>,
    pub source_url: Option<String>,
    pub doi: Option<String>,
    pub arxiv_id: Option<String>,
    pub pdf_path: Option<String>,
    pub publish_date: Option<String>,
    pub venue: Option<String>,
    pub is_read: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: TagId,
    pub name: String,
    pub parent_id: Option<TagId>,
    pub aliases: Vec<String>,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paper_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: NoteId,
    pub paper_id: PaperId,
    pub content: String,
    pub note_type: NoteType,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NoteType {
    Summary,
    Method,
    Result,
    Thought,
    General,
}

impl std::fmt::Display for NoteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl NoteType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Method => "method",
            Self::Result => "result",
            Self::Thought => "thought",
            Self::General => "general",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "summary" => Some(Self::Summary),
            "method" => Some(Self::Method),
            "result" => Some(Self::Result),
            "thought" => Some(Self::Thought),
            "general" => Some(Self::General),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: GroupId,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationType {
    Cites,
    RelatedTo,
    Contrasts,
    Extends,
    Improves,
}

impl RelationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cites => "cites",
            Self::RelatedTo => "related",
            Self::Contrasts => "contrasts",
            Self::Extends => "extends",
            Self::Improves => "improves",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "cites" => Some(Self::Cites),
            "related" => Some(Self::RelatedTo),
            "contrasts" => Some(Self::Contrasts),
            "extends" => Some(Self::Extends),
            "improves" => Some(Self::Improves),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub from_id: PaperId,
    pub to_id: PaperId,
    pub relation: RelationType,
    pub strength: u8,
    pub note: Option<String>,
}

// ── 操作参数 ──

/// 添加论文的参数
#[derive(Default)]
pub struct AddPaperParams {
    pub title: String,
    pub authors: Option<Vec<String>>,
    pub abstract_text: Option<String>,
    pub source_url: Option<String>,
    pub doi: Option<String>,
    pub arxiv_id: Option<String>,
    pub pdf_path: Option<String>,
    pub publish_date: Option<NaiveDate>,
    pub venue: Option<String>,
    pub force: bool, // 跳过查重
}

/// 更新论文的参数（所有字段可选，仅更新提供的字段）
#[derive(Default)]
pub struct UpdatePaperParams {
    pub title: Option<String>,
    pub authors: Option<Vec<String>>,
    pub abstract_text: Option<String>,
    pub source_url: Option<String>,
    pub doi: Option<String>,
    pub arxiv_id: Option<String>,
    pub pdf_path: Option<String>,
    pub publish_date: Option<NaiveDate>,
    pub venue: Option<String>,
    pub is_read: Option<bool>,
}

/// 列表查询参数
pub struct ListParams {
    pub page: u32,
    pub page_size: u32,
    pub tag: Option<String>,
    pub group: Option<String>,
    pub is_read: Option<bool>,
    pub sort: SortField,
}

impl Default for ListParams {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 10,
            tag: None,
            group: None,
            is_read: None,
            sort: SortField::Created,
        }
    }
}

#[derive(Default)]
pub enum SortField {
    #[default]
    Created,
    Title,
    Date,
}

/// 分页查询结果
pub struct ListResult {
    pub papers: Vec<Paper>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

/// 查重结果
pub enum DedupResult {
    New(Paper),
    Duplicate { existing: Paper, matched_by: String },
}
