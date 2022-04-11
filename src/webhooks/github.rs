use rocket::{http::Status, State};
use tracing::{info, trace, warn};

mod events;
pub use events::*;

mod signing;
use signing::SignedGitHubPayload;

use crate::webhooks::{Event, EventSender};

pub const X_GITHUB_EVENT: &str = "X-GitHub-Event";

#[rocket::post("/api/webhooks/github", data = "<payload>")]
pub fn github_webhook(
    event: GitHubEventType,
    payload: SignedGitHubPayload,
    sender: &State<EventSender>,
) -> Status {
    info!("received event {:?} with signed payload", event);
    trace!("payload: {}", payload.0);

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
