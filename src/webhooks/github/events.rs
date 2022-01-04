use std::fmt::Display;

use serde::Deserialize;
use url::Url;

use crate::bot::utils::shorten_content;

mod commit_comment;
mod create;
mod fork;
mod issue_comment;
mod issues;
mod membership;
mod organization;
mod ping;
mod pull_request;
mod pull_request_review;
mod pull_request_review_comment;
mod push;
mod repository;
mod types;

pub use commit_comment::*;
pub use create::*;
pub use fork::*;
pub use issue_comment::*;
pub use issues::*;
pub use membership::*;
pub use organization::*;
pub use ping::*;
pub use pull_request::*;
pub use pull_request_review::*;
pub use pull_request_review_comment::*;
pub use push::*;
pub use repository::*;
pub use types::*;

#[derive(Debug)]
pub enum GitHubEvent {
    CommitComment(CommitCommentEvent),
    Create(CreateEvent),
    Fork(ForkEvent),
    IssueComment(IssueCommentEvent),
    Issues(IssuesEvent),
    Membership(MembershipEvent),
    Organization(OrganizationEvent),
    Ping(PingEvent),
    PullRequest(PullRequestEvent),
    PullRequestReview(PullRequestReviewEvent),
    PullRequestReviewComment(PullRequestReviewCommentEvent),
    Push(PushEvent),
    Repository(RepositoryEvent),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RefType {
    Branch,
    Tag,
}

#[derive(Debug, Deserialize)]
pub struct GitHubUser {
    pub login: String,
    pub id: u64,
}

#[derive(Debug, Deserialize)]
pub struct Repository {
    pub name: String,
    pub full_name: String,
    pub html_url: Url,
}

impl Repository {
    pub fn ref_url(&self, r#ref: &str) -> Result<Url, url::ParseError> {
        Url::parse(&format!(
            "https://github.com/{}/tree/{}",
            self.full_name, r#ref
        ))
    }
}

#[derive(Debug, Deserialize)]
pub struct Issue {
    pub number: u64,
    pub html_url: Url,
    pub title: String,
    pub milestone: Option<Milestone>,
    // an issue can be a PR, in this case the object contains a `pull_request` key with urls to the
    // PR
    pub pull_request: Option<PullRequestLinks>,
}

impl Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{} ({})", self.number, shorten_content(&self.title))
    }
}

#[derive(Debug, Deserialize)]
pub struct Milestone {
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct Comment {
    pub html_url: Url,
    pub body: String,
    pub commit_id: Option<String>,
    pub pull_request_review_id: Option<u64>,
    pub path: Option<String>,
    pub position: Option<u64>,
}

impl Comment {
    pub fn location(&self) -> Option<String> {
        self.path.as_ref().map(|path| {
            format!(
                "on file {} @ {}",
                path,
                self.position
                    .expect("comment on file without specific position"),
            )
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub html_url: Url,
    pub title: String,
    pub user: GitHubUser,
    pub requested_reviewers: Vec<GitHubUser>,
    pub base: PrRef,
    pub head: PrRef,
    pub merged: Option<bool>,
}

impl Display for PullRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PR #{}: {} by {}",
            self.number,
            shorten_content(&self.title),
            self.user.login
        )
    }
}

#[derive(Debug, Deserialize)]
pub struct PrRef {
    pub r#ref: String,
}

#[derive(Debug, Deserialize)]
pub struct PullRequestLinks {
    pub html_url: Url,
}

#[derive(Debug, Deserialize)]
pub struct Team {
    pub name: String,
    pub id: u64,
    pub description: String,
    pub privacy: String,
    pub permission: String,
    pub html_url: Url,
}
