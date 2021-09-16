use serde::Deserialize;

use crate::webhooks::github::events::{GitHubUser, Issue, Repository};

#[derive(Debug, Deserialize)]
pub struct IssuesEvent {
    pub repository: Repository,
    pub sender: GitHubUser,
    pub issue: Issue,
    pub changes: Option<IssueChanges>,
    pub assignee: Option<GitHubUser>,
    pub action: String,
}

#[derive(Debug, Deserialize)]
pub struct IssueChanges {
    pub title: Option<IssueChangesFrom>,
    pub body: Option<IssueChangesFrom>,
}

#[derive(Debug, Deserialize)]
pub struct IssueChangesFrom {
    pub from: String,
}
