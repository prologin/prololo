use serde::Deserialize;

use crate::webhooks::github::events::GitHubUser;

#[derive(Debug, Deserialize)]
pub struct OrganizationEvent {
    pub action: String,
    pub sender: GitHubUser,

    // When 'invitation', 'user' should be set
    pub invitation: Option<OrganizationInvitation>,
    pub user: Option<GitHubUser>,

    // Otherwise, 'user' is accessed through 'membership'
    pub membership: Option<OrganizationMembership>,
}

#[derive(Debug, Deserialize)]
pub struct OrganizationInvitation {
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct OrganizationMembership {
    pub role: String,
    pub user: GitHubUser,
}
