use serde::Deserialize;
use url::Url;

use crate::webhooks::github::events::{GitHubUser, Repository};

#[derive(Debug, Deserialize)]
pub struct PushEvent {
    pub repository: Repository,
    pub sender: GitHubUser,
    pub commits: Vec<Commit>,
    pub head_commit: Option<Commit>,
    pub forced: bool,
    pub created: bool,
    pub r#ref: String,
    pub compare: Url,
}

#[derive(Debug, Deserialize)]
pub struct Commit {
    pub id: String,
    pub url: Url,
    pub distinct: bool,
    pub message: String,
}

impl Commit {
    pub fn title(&self) -> &str {
        self.message
            .lines()
            .next()
            .expect("body has at least one line")
    }
}
