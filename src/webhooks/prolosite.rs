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
use url::Url;

use crate::webhooks::{Event, EventSender};

const AUTHORIZATION: &str = "Authorization";

#[derive(Debug)]
pub enum ProloSiteEvent {
    Error(DjangoErrorPayload),
    Forum(ForumPayload),
    NewSchool(NewSchoolPayload),
    Impersonate(ImpersonatePayload),
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
pub(crate) fn forum(
    _token: AuthorizationHeader,
    payload: Json<ForumPayload>,
    sender: &State<EventSender>,
) {
    info!("received forum update");
    trace!("payload: {:?}", payload.0);

    sender
        .0
        .send(Event::ProloSite(ProloSiteEvent::Forum(
            payload.into_inner(),
        )))
        .expect("mspc channel was closed / dropped");
}

#[rocket::post(
    "/api/webhooks/prolosite/new-school",
    format = "json",
    data = "<payload>"
)]
pub(crate) fn new_school(
    _token: AuthorizationHeader,
    payload: Json<NewSchoolPayload>,
    sender: &State<EventSender>,
) {
    info!("received new school update");
    trace!("payload: {:?}", payload.0);

    sender
        .0
        .send(Event::ProloSite(ProloSiteEvent::NewSchool(
            payload.into_inner(),
        )))
        .expect("mspc channel was closed / dropped");
}

#[rocket::post(
    "/api/webhooks/prolosite/impersonate",
    format = "json",
    data = "<payload>"
)]
pub(crate) fn impersonate(
    _token: AuthorizationHeader,
    payload: Json<ImpersonatePayload>,
    sender: &State<EventSender>,
) {
    info!("received impersonate notice");
    trace!("payload: {:?}", payload.0);

    sender
        .0
        .send(Event::ProloSite(ProloSiteEvent::Impersonate(
            payload.into_inner(),
        )))
        .expect("mspc channel was closed / dropped");
}

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
    #[allow(dead_code)]
    pub(crate) trace: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ForumPayload {
    pub(crate) username: String,
    pub(crate) forum: String,
    pub(crate) title: String,
    pub(crate) url: Url,
}

#[derive(Debug, Deserialize)]
pub struct NewSchoolPayload {
    pub(crate) name: String,
    pub(crate) url: Url,
}

#[derive(Debug, Deserialize)]
pub struct ImpersonatePayload {
    pub(crate) event: String,
    pub(crate) hijacker: User,
    pub(crate) hijacked: User,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub(crate) username: String,
    pub(crate) url: Url,
}
