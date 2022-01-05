use serde::Deserialize;

use crate::webhooks::github::events::{GitHubUser, Repository};

#[derive(Debug, Deserialize)]
pub struct PingEvent {
    pub zen: String,
    pub repository: Option<Repository>,
    pub sender: GitHubUser,
}
