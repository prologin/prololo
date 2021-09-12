use std::io;

use anyhow::anyhow;
use rocket::{
    data::{ByteUnit, FromData, Outcome},
    http::{ContentType, Status},
    Data, Request, State,
};
use tracing::trace;

use crate::webhooks::github::GitHubSecret;

const X_GITHUB_SIGNATURE: &str = "X-Hub-Signature-256";

fn validate_signature(secret: &str, signature: &str, data: &str) -> bool {
    trace!("validating signature...");
    use hmac::{Hmac, Mac, NewMac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("this should never fail");

    mac.update(data.as_bytes());

    // GitHub puts a prefix in front of its hex SHA256
    let signature = match signature.strip_prefix("sha256=") {
        Some(s) => s,
        None => {
            trace!("couldn't strip prefix from signature `{}`", signature);
            return false;
        }
    };

    match hex::decode(signature) {
        Ok(bytes) => mac.verify(&bytes).is_ok(),
        Err(_) => {
            trace!("couldn't decode hex-encoded signature {}", signature);
            false
        }
    }
}

pub struct SignedGitHubPayload(pub String);

const LIMIT: ByteUnit = ByteUnit::Mebibyte(1);

// Tracking issue for chaining Data guards to avoid reimplementing all this:
// https://github.com/SergioBenitez/Rocket/issues/775
#[rocket::async_trait]
impl<'r> FromData<'r> for SignedGitHubPayload {
    type Error = anyhow::Error;

    async fn from_data(request: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
        trace!("received payload on GitHub webhook endpoint: {:?}", request);

        let json_ct = ContentType::new("application", "json");
        if request.content_type() != Some(&json_ct) {
            trace!(
                "content type `{:?}` wasn't json, stopping here...",
                request.content_type()
            );
            return Outcome::Failure((Status::BadRequest, anyhow!("wrong content type")));
        }

        let signatures = request
            .headers()
            .get(X_GITHUB_SIGNATURE)
            .collect::<Vec<_>>();
        if signatures.len() != 1 {
            trace!("couldn't locate {} header", X_GITHUB_SIGNATURE);
            return Outcome::Failure((
                Status::BadRequest,
                anyhow!("request header needs exactly one signature"),
            ));
        }

        let size_limit = request.limits().get("json").unwrap_or(LIMIT);
        let content = match data.open(size_limit).into_string().await {
            Ok(s) if s.is_complete() => s.into_inner(),
            Ok(_) => {
                let eof = io::ErrorKind::UnexpectedEof;
                trace!("payload was too big");
                return Outcome::Failure((
                    Status::PayloadTooLarge,
                    io::Error::new(eof, "data limit exceeded").into(),
                ));
            }
            Err(e) => return Outcome::Failure((Status::BadRequest, e.into())),
        };

        let signature = signatures[0];
        let secret = request.guard::<&State<GitHubSecret>>().await.unwrap();

        if !validate_signature(&secret.0, signature, &content) {
            trace!("signature validation failed, stopping here...");
            return Outcome::Failure((Status::BadRequest, anyhow!("couldn't verify signature")));
        }

        trace!("validated GitHub payload");
        Outcome::Success(SignedGitHubPayload(content))
    }
}
