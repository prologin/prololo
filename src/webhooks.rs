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
        let prololo_config = request.guard::<&State<ProloloConfig>>().await.unwrap();

        let endpoint_segments: Vec<&str> = request.uri().path().segments().skip(2).collect();
        let auth_secret = match endpoint_segments[0] {
            "prolosite" => Some(prololo_config.prolosite_secret.as_str()),
            "generic" => match &prololo_config.generic_endpoints.get(endpoint_segments[1]) {
                Some(endpoint) => Some(endpoint.secret.as_str()),
                None => {
                    return Outcome::Failure((
                        Status::NotFound,
                        anyhow!("no endpoint named '{}'", endpoint_segments[1]),
                    ))
                }
            },
            _ => unreachable!(),
        };

        if Some(authorization) != auth_secret {
            trace!("secret validation failed, stopping here...");
            return Outcome::Failure((Status::BadRequest, anyhow!("secret doesn't match")));
        }

        trace!("validated request");
        Outcome::Success(AuthorizationHeader(authorization))
    }
}
