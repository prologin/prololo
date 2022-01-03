use serde::Deserialize;

use crate::webhooks::github::events::{Comment, GitHubUser, Repository};

#[derive(Debug, Deserialize)]
pub struct CommitCommentEvent {
    pub sender: GitHubUser,
    pub repository: Repository,
    pub comment: Comment,
}
