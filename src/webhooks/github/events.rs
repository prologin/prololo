use serde::Deserialize;
use url::Url;

mod create;
mod types;

pub use create::*;
pub use types::*;

#[derive(Debug)]
pub enum GitHubEvent {
    Create(CreateEvent),
    Issues,
    IssueComment,
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
