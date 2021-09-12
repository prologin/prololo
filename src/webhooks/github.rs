use anyhow::anyhow;
use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};
use serde::Deserialize;

mod signing;
use signing::SignedGitHubPayload;
use tracing::{debug, info, warn};

const X_GITHUB_EVENT: &str = "X-GitHub-Event";

struct GitHubSecret(String);

#[rocket::post("/api/webhooks/github", data = "<payload>")]
pub fn github_webhook(event: GitHubEventType, payload: SignedGitHubPayload) -> &'static str {
    info!(
        "received event {:?} with signed payload:\n{}",
        event, payload.0
    );

    "OK"
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

enum GitHubEvent {
    Create { ref_type: RefType },
    Issues,
    IssueComment,
    Push,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum RefType {
    Branch,
    Tag,
}
