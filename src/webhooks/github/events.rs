use std::fmt::Display;

use serde::Deserialize;
use url::Url;

mod create;
mod issue_comment;
mod issues;
mod types;

pub use create::*;
pub use issue_comment::*;
pub use issues::*;
pub use types::*;

#[derive(Debug)]
pub enum GitHubEvent {
    Create(CreateEvent),
    IssueComment(IssueCommentEvent),
    Issues(IssuesEvent),
    Push,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RefType {
    Branch,
    Tag,
}

#[derive(Debug, Deserialize)]
pub struct GitHubUser {
    pub login: String,
    pub id: u64,
}

#[derive(Debug, Deserialize)]
pub struct Repository {
    pub name: String,
    pub full_name: String,
    pub html_url: Url,
}

impl Repository {
    pub fn ref_url(&self, r#ref: &str) -> String {
        format!("https://github.com/{}/tree/{}", self.full_name, r#ref)
    }
}

#[derive(Debug, Deserialize)]
pub struct Issue {
    pub number: u64,
    pub html_url: Url,
    pub title: String,
    pub milestone: Option<Milestone>,
}

impl Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{} ({})", self.number, self.title)
    }
}

#[derive(Debug, Deserialize)]
pub struct Milestone {
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct Comment {
    pub html_url: Url,
    pub body: String,
}
