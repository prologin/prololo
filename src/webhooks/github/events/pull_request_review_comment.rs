use serde::Deserialize;

use crate::webhooks::github::events::{Comment, GitHubUser, PullRequest, Repository};

#[derive(Debug, Deserialize)]
pub struct PullRequestReviewCommentEvent {
    pub repository: Repository,
    pub sender: GitHubUser,
    pub pull_request: PullRequest,
    pub action: String,
    pub comment: Comment,
}
