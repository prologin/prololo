use serde::Deserialize;

use crate::webhooks::github::events::{GitHubUser, Repository};

#[derive(Debug, Deserialize)]
pub struct ForkEvent {
    pub forkee: Repository,
    pub repository: Repository,
    pub sender: GitHubUser,
}
