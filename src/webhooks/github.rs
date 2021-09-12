use anyhow::{anyhow, bail};
use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request, State,
};
use serde::Deserialize;

mod signing;
use signing::SignedGitHubPayload;
use tracing::{debug, info, warn};
use url::Url;

use crate::webhooks::{Event, EventSender};

const X_GITHUB_EVENT: &str = "X-GitHub-Event";

struct GitHubSecret(String);

#[rocket::post("/api/webhooks/github", data = "<payload>")]
pub fn github_webhook(
    event: GitHubEventType,
    payload: SignedGitHubPayload,
    sender: &State<EventSender>,
) -> Status {
    info!(
        "received event {:?} with signed payload:\n{}",
        event, payload.0
    );

    let event = match event.parse_payload(&payload) {
        Ok(event) => event,
        Err(e) => {
            warn!(
                "couldn't parse payload for event {:?}: {}\n{}",
                event, e, payload.0
            );
            return Status::BadRequest;
        }
    };

    sender
        .0
        .send(Event::GitHub(event))
        .expect("mpsc channel was closed / dropped");

    Status::Ok
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitHubEventType {
    Create,
    Issues,
    IssueComment,
    Push,
    Unknown,
}

impl GitHubEventType {
    fn parse_payload(&self, payload: &SignedGitHubPayload) -> anyhow::Result<GitHubEvent> {
        Ok(match self {
            Self::Create => GitHubEvent::Create(serde_json::from_str(&payload.0)?),
            Self::Unknown => bail!("unknown event type"),
            _ => unimplemented!(),
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

#[derive(Debug)]
pub enum GitHubEvent {
    Create(CreateEvent),
    Issues,
    IssueComment,
    Push,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RefType {
    Branch,
    Tag,
}

#[derive(Debug, Deserialize)]
pub struct GitHubUser {
    login: String,
}

#[derive(Debug, Deserialize)]
pub struct Repository {
    name: String,
    full_name: String,
    html_url: Url,
}

#[derive(Debug, Deserialize)]
pub struct CreateEvent {
    r#ref: String,
    ref_type: RefType,
    repository: Repository,
    sender: GitHubUser,
}
