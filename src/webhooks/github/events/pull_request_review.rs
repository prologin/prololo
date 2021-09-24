use serde::Deserialize;
use url::Url;

use crate::webhooks::github::events::{GitHubUser, PullRequest, Repository};

#[derive(Debug, Deserialize)]
pub struct PullRequestReviewEvent {
    pub repository: Repository,
    pub sender: GitHubUser,
    pub pull_request: PullRequest,
    pub review: Review,
    pub action: String,
}

#[derive(Debug, Deserialize)]
pub struct Review {
    pub state: String,
    pub user: GitHubUser,
    pub html_url: Url,
}
