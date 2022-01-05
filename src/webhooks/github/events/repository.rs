use serde::Deserialize;

use crate::webhooks::github::events::{GitHubUser, Repository};

#[derive(Debug, Deserialize)]
pub struct RepositoryEvent {
    pub action: String,
    pub repository: Repository,
    pub sender: GitHubUser,
    pub changes: Option<RepositoryChanges>,
}

#[derive(Debug, Deserialize)]
pub struct RepositoryChanges {
    pub repository: RepositoryChangesName,
}

#[derive(Debug, Deserialize)]
pub struct RepositoryChangesName {
    pub name: RepositoryChangesNameFrom,
}

#[derive(Debug, Deserialize)]
pub struct RepositoryChangesNameFrom {
    pub from: String,
}
