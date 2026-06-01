use askama::Template;
use refstore_core::model::{Note, Paper};

use crate::i18n::Translations;

#[derive(Template)]
#[template(path = "list.html")]
pub struct ListTemplate {
    pub papers: Vec<PaperView>,
    pub page: u32,
    pub total_pages: u32,
    pub total: i64,
    pub lang: String,
    pub i18n: Translations,
}

pub struct PaperView {
    pub id: String,
    pub title: String,
    pub authors_str: String,
    pub tags_str: String,
    pub is_read: bool,
}

#[derive(Template)]
#[template(path = "detail.html")]
pub struct DetailTemplate {
    pub paper: Paper,
    pub tags_str: String,
    pub notes: Vec<Note>,
    pub citations: Vec<CitationView>,
    pub has_graph: bool,
    pub lang: String,
    pub i18n: Translations,
}

pub struct CitationView {
    pub is_from: bool,
    pub source_id: String,
    pub source_title: String,
    pub target_id: String,
    pub target_title: String,
    pub relation: String,
    pub direction: String,
}

#[derive(Template)]
#[template(path = "tags.html")]
pub struct TagsTemplate {
    pub tags: Vec<refstore_core::model::Tag>,
    pub lang: String,
    pub i18n: Translations,
}

#[derive(Template)]
#[template(path = "groups.html")]
pub struct GroupsTemplate {
    pub groups: Vec<(refstore_core::model::Group, i64)>,
    pub lang: String,
    pub i18n: Translations,
}
