use serde::Deserialize;

use crate::webhooks::github::events::{GitHubUser, PullRequest, Repository};

#[derive(Debug, Deserialize)]
pub struct PullRequestEvent {
    pub repository: Repository,
    pub sender: GitHubUser,
    pub pull_request: PullRequest,
    pub assignee: Option<GitHubUser>,
    pub action: String,
}
