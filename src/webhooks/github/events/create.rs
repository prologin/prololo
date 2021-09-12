use serde::Deserialize;

use crate::webhooks::github::events::{GitHubUser, RefType, Repository};

#[derive(Debug, Deserialize)]
pub struct CreateEvent {
    pub r#ref: String,
    pub ref_type: RefType,
    pub repository: Repository,
    pub sender: GitHubUser,
}
