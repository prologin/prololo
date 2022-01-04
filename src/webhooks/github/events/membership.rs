use serde::Deserialize;

use crate::webhooks::github::events::{GitHubUser, Team};

#[derive(Debug, Deserialize)]
pub struct MembershipEvent {
    pub action: String,
    pub member: GitHubUser,
    pub team: Team,
    pub sender: GitHubUser,
}
