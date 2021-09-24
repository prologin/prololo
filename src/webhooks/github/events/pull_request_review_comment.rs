use serde::Deserialize;
use url::Url;

use crate::webhooks::github::events::{GitHubUser, PullRequest, Repository};

#[derive(Debug, Deserialize)]
pub struct PullRequestReviewCommentEvent {
    pub repository: Repository,
    pub sender: GitHubUser,
    pub pull_request: PullRequest,
    pub action: String,
    pub comment: ReviewComment,
}

#[derive(Debug, Deserialize)]
pub struct ReviewComment {
    pub pull_request_review_id: Option<u64>,
    pub html_url: Url,
    pub path: Option<String>,
    pub position: Option<u64>,
}

impl ReviewComment {
    pub fn location(&self) -> String {
        match &self.path {
            Some(path) => format!(
                "on file {} @ {}",
                path,
                self.position
                    .expect("comment on file without specific position"),
            ),
            None => String::new(),
        }
    }
}
