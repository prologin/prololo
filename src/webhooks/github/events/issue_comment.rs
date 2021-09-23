use serde::Deserialize;

use crate::webhooks::github::events::{Comment, GitHubUser, Issue, Repository};

#[derive(Debug, Deserialize)]
pub struct IssueCommentEvent {
    pub sender: GitHubUser,
    pub repository: Repository,
    pub issue: Issue,
    pub action: String,
    pub comment: Comment,
}
