use std::io;
use std::ops::{Deref, DerefMut};

use anyhow::anyhow;
use rocket::{
    data::{ByteUnit, FromData, Outcome},
    http::{ContentType, Status},
    Data, Request, State,
};

use crate::webhooks::github::GitHubSecret;

const X_GITHUB_SIGNATURE: &str = "X-Hub-Signature-256";

fn validate_signature(secret: &str, signature: &str, data: &str) -> bool {
    use hmac::{Hmac, Mac, NewMac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("this should never fail");

    mac.update(data.as_bytes());

    match hex::decode(signature) {
        Ok(bytes) => mac.verify(&bytes).is_ok(),
        Err(_) => false,
    }
}

pub struct SignedGitHubPayload(pub String);

// FIXME: probably not needed
impl Deref for SignedGitHubPayload {
    type Target = String;

    fn deref(&self) -> &String {
        &self.0
    }
}

impl DerefMut for SignedGitHubPayload {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

const LIMIT: ByteUnit = ByteUnit::Mebibyte(1);

// Tracking issue for chaining Data guards to avoid reimplementing all this:
// https://github.com/SergioBenitez/Rocket/issues/775
#[rocket::async_trait]
impl<'r> FromData<'r> for SignedGitHubPayload {
    type Error = anyhow::Error;

    async fn from_data(request: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
        let json_ct = ContentType::new("application", "json");
        if request.content_type() != Some(&json_ct) {
            return Outcome::Failure((Status::BadRequest, anyhow!("wrong content type")));
        }

        let signatures = request
            .headers()
            .get(X_GITHUB_SIGNATURE)
            .collect::<Vec<_>>();
        if signatures.len() != 1 {
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
            return Outcome::Failure((Status::BadRequest, anyhow!("couldn't verify signature")));
        }

        Outcome::Success(SignedGitHubPayload(content))
    }
}
