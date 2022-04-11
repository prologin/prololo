use rocket::{serde::json::Json, State};
use serde::Deserialize;
use tracing::{debug, trace};
use url::Url;

use crate::webhooks::{Event, EventSender, GenericAuthorize};

#[derive(Debug)]
pub struct GenericEvent(pub GenericPayload);

#[rocket::post(
    "/api/webhooks/generic/<endpoint>",
    format = "json",
    data = "<payload>"
)]
pub(crate) fn generic(
    endpoint: String,
    _token: GenericAuthorize,
    payload: Json<GenericPayload>,
    sender: &State<EventSender>,
) {
    debug!("received request on endpoint '{}'", endpoint);
    trace!("payload: {:?}", payload.0);

    sender
        .0
        .send(Event::Generic(GenericEvent(payload.into_inner())))
        .expect("mspc channel was closed / dropped");
}

#[derive(Debug, Deserialize)]
pub struct GenericPayload {
    pub(crate) tag: Option<String>,
    pub(crate) message: String,
    pub(crate) url: Option<Url>,
}
