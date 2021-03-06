use anyhow::{anyhow, bail};
use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};
use serde::Deserialize;
use tracing::{debug, warn};

use crate::webhooks::github::{GitHubEvent, SignedGitHubPayload, X_GITHUB_EVENT};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitHubEventType {
    CommitComment,
    Create,
    Fork,
    IssueComment,
    Issues,
    Membership,
    Organization,
    Ping,
    PullRequest,
    PullRequestReview,
    PullRequestReviewComment,
    Push,
    Repository,
    Unknown,
}

impl GitHubEventType {
    pub(crate) fn parse_payload(
        &self,
        payload: &SignedGitHubPayload,
    ) -> anyhow::Result<GitHubEvent> {
        Ok(match self {
            Self::CommitComment => GitHubEvent::CommitComment(serde_json::from_str(&payload.0)?),
            Self::Create => GitHubEvent::Create(serde_json::from_str(&payload.0)?),
            Self::Fork => GitHubEvent::Fork(serde_json::from_str(&payload.0)?),
            Self::IssueComment => GitHubEvent::IssueComment(serde_json::from_str(&payload.0)?),
            Self::Issues => GitHubEvent::Issues(serde_json::from_str(&payload.0)?),
            Self::Membership => GitHubEvent::Membership(serde_json::from_str(&payload.0)?),
            Self::Organization => GitHubEvent::Organization(serde_json::from_str(&payload.0)?),
            Self::Ping => GitHubEvent::Ping(serde_json::from_str(&payload.0)?),
            Self::PullRequest => GitHubEvent::PullRequest(serde_json::from_str(&payload.0)?),
            Self::PullRequestReview => {
                GitHubEvent::PullRequestReview(serde_json::from_str(&payload.0)?)
            }
            Self::PullRequestReviewComment => {
                GitHubEvent::PullRequestReviewComment(serde_json::from_str(&payload.0)?)
            }
            Self::Push => GitHubEvent::Push(serde_json::from_str(&payload.0)?),
            Self::Repository => GitHubEvent::Repository(serde_json::from_str(&payload.0)?),
            Self::Unknown => bail!("unknown event type"),
        })
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for GitHubEventType {
    type Error = anyhow::Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let event_types = request.headers().get(X_GITHUB_EVENT).collect::<Vec<_>>();
        if event_types.len() != 1 {
            return Outcome::Failure((
                Status::BadRequest,
                anyhow!("request header needs exactly one event type"),
            ));
        }

        let event_type = event_types[0];

        // HACK: serialize the Rust String to a JSON string so that it's deserializable into the
        // GitHubEventType enum correctly:
        //
        // - `create` is not a valid JSON string
        // - `"create"` is!
        let event_type_json_value =
            serde_json::to_value(event_type).expect("`String` serialization should never fail");
        let event_type = match serde_json::from_value::<GitHubEventType>(event_type_json_value) {
            Ok(ev_type) => ev_type,
            Err(e) => {
                warn!("received unknown event type: {}, {}", event_type, e);
                GitHubEventType::Unknown
            }
        };

        debug!("received request with type {:?}", event_type);

        Outcome::Success(event_type)
    }
}
