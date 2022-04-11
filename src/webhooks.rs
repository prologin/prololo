use anyhow::anyhow;
use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    State,
};
use tokio::sync::mpsc::UnboundedSender;

use tracing::trace;

pub mod github;
pub use github::{github_webhook, GitHubEvent};

pub mod prolosite;
pub(crate) use prolosite::ProloSiteEvent;

pub mod generic;
pub(crate) use generic::GenericEvent;

use crate::config::ProloloConfig;

pub struct EventSender(pub UnboundedSender<Event>);

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum Event {
    GitHub(GitHubEvent),
    ProloSite(ProloSiteEvent),
    Generic(GenericEvent),
}

const AUTHORIZATION: &str = "Authorization";

fn get_auth_token<'r>(request: &'r rocket::Request<'_>) -> Option<&'r str> {
    let authorization = request.headers().get(AUTHORIZATION).collect::<Vec<_>>();

    if authorization.len() != 1 {
        trace!("couldn't locate {} header", AUTHORIZATION);
        None
    } else {
        Some(authorization[0])
    }
}

macro_rules! authorize_or_error {
    ($auth_type:ident, $authorization:expr, $auth_secret:expr) => {
        if $authorization != $auth_secret {
            trace!("secret validation failed, stopping here...");
            return Outcome::Failure((Status::BadRequest, anyhow!("secret doesn't match")));
        } else {
            trace!("validated request");
            Outcome::Success($auth_type($authorization))
        }
    };
}

macro_rules! missing_auth {
    () => {
        Outcome::Failure((
            Status::BadRequest,
            anyhow!("request needs an authorization header"),
        ))
    };
}

pub(crate) struct ProlositeAuthorize<'r>(&'r str);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ProlositeAuthorize<'r> {
    type Error = anyhow::Error;

    async fn from_request(request: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(authorization) = get_auth_token(request) {
            let auth_secret = request
                .guard::<&State<ProloloConfig>>()
                .await
                .unwrap()
                .prolosite_secret
                .as_str();

            authorize_or_error!(ProlositeAuthorize, authorization, auth_secret)
        } else {
            missing_auth!()
        }
    }
}

pub(crate) struct GenericAuthorize<'r>(&'r str);
#[rocket::async_trait]
impl<'r> FromRequest<'r> for GenericAuthorize<'r> {
    type Error = anyhow::Error;

    async fn from_request(request: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(authorization) = get_auth_token(request) {
            let prololo_config = request.guard::<&State<ProloloConfig>>().await.unwrap();

            let endpoint: &str = request
                .uri()
                .path()
                .segments()
                .skip(3)
                .nth(0)
                .expect("should never happen");
            let auth_secret = match &prololo_config.generic_endpoints.get(endpoint) {
                Some(endpoint) => endpoint.secret.as_str(),
                None => {
                    return Outcome::Failure((
                        Status::NotFound,
                        anyhow!("no endpoint named '{}'", endpoint),
                    ))
                }
            };

            authorize_or_error!(GenericAuthorize, authorization, auth_secret)
        } else {
            missing_auth!()
        }
    }
}
