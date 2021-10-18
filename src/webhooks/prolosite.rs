use std::path::PathBuf;

use anyhow::anyhow;
use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    serde::json::Json,
    State,
};
use serde::Deserialize;
use tracing::{info, trace};

use crate::webhooks::{Event, EventSender};

const AUTHORIZATION: &str = "Authorization";

#[derive(Debug)]
pub enum ProloSiteEvent {
    Error(DjangoErrorPayload),
}

pub struct ProlositeSecret(pub String);

pub(crate) struct AuthorizationHeader<'r>(&'r str);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthorizationHeader<'r> {
    type Error = anyhow::Error;

    async fn from_request(request: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        let authorization = request.headers().get(AUTHORIZATION).collect::<Vec<_>>();
        if authorization.len() != 1 {
            trace!("couldn't locate {} header", AUTHORIZATION);
            return Outcome::Failure((
                Status::BadRequest,
                anyhow!("request needs an authorization header"),
            ));
        }
        let authorization = authorization[0];
        let auth_secret = &request.guard::<&State<ProlositeSecret>>().await.unwrap().0;

        if authorization != auth_secret {
            trace!("secret validation failed, stopping here...");
            return Outcome::Failure((Status::BadRequest, anyhow!("secret doesn't match")));
        }

        trace!("validated Prolosite request");
        Outcome::Success(AuthorizationHeader(authorization))
    }
}

#[rocket::post("/api/webhooks/prolosite/django", format = "json", data = "<payload>")]
pub(crate) fn django(
    _token: AuthorizationHeader,
    payload: Json<DjangoErrorPayload>,
    sender: &State<EventSender>,
) {
    info!("received django error");
    trace!("payload: {:?}", payload.0);
    sender
        .0
        .send(Event::ProloSite(ProloSiteEvent::Error(
            payload.into_inner(),
        )))
        .expect("mspc channel was closed / dropped");
}

#[rocket::post("/api/webhooks/prolosite/forum", format = "json", data = "<payload>")]
pub(crate) fn forum(_token: AuthorizationHeader, payload: Json<ForumPayload>) {}

#[rocket::post(
    "/api/webhooks/prolosite/new-school",
    format = "json",
    data = "<payload>"
)]
pub(crate) fn new_school(_token: AuthorizationHeader, payload: Json<NewSchoolPayload>) {}

#[derive(Debug, Deserialize)]
pub struct DjangoErrorPayload {
    pub(crate) request: Request,
    pub(crate) exception: Exception,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Request {
    pub(crate) user: Option<String>,
    pub(crate) method: String,
    pub(crate) path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Exception {
    pub(crate) value: String,
    pub(crate) trace: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ForumPayload {}

#[derive(Debug, Deserialize)]
pub struct NewSchoolPayload {}
