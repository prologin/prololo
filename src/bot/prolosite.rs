use std::fmt::Write;

use tracing::trace;

use crate::{
    bot::{emoji, message_builder::MessageBuilder, utils::shorten_content_length, Response},
    webhooks::{
        prolosite::{DjangoErrorPayload, ForumPayload, ImpersonatePayload, NewSchoolPayload},
        ProloSiteEvent,
    },
};

pub(crate) fn handle_prolosite_event(event: ProloSiteEvent) -> anyhow::Result<Option<Response>> {
    trace!("handling prolosite event");
    let response = match event {
        ProloSiteEvent::Error(event) => handle_prolosite_error(event),
        ProloSiteEvent::Forum(event) => handle_prolosite_forum(event),
        ProloSiteEvent::NewSchool(event) => handle_prolosite_new_school(event),
        ProloSiteEvent::Impersonate(event) => handle_prolosite_impersonate(event),
    };

    Ok(response)
}

fn handle_prolosite_error(event: DjangoErrorPayload) -> Option<Response> {
    let mut message = MessageBuilder::new();

    message.tag("django crash", Some(emoji::FIRE));

    if let Some(user) = event.request.user {
        write!(message, " ({})", user).unwrap();
    }

    let method = &event.request.method;
    write!(message, " {} ", method).unwrap();

    let path = event.request.path.display();
    message.code();
    write!(message, "{}", path).unwrap();
    message.close_last();

    write!(message, ": ").unwrap();

    // TODO: parse trace and show fancier exceptions
    let exception = &event.exception.value;
    message.code();
    write!(message, "{}", exception).unwrap();
    message.close_last();

    Some(Response {
        message,
        repo: None,
    })
}

fn handle_prolosite_forum(event: ForumPayload) -> Option<Response> {
    let mut message = MessageBuilder::new();

    message.tag("forum", Some(emoji::SPEECH_BALLOON));

    write!(message, " {} created ", event.username).unwrap();

    message.main_link("new thread", &event.url);

    write!(
        message,
        " in {}: {}",
        event.forum,
        shorten_content_length(&event.title, 140)
    )
    .unwrap();

    Some(Response {
        message,
        repo: None,
    })
}

fn handle_prolosite_new_school(event: NewSchoolPayload) -> Option<Response> {
    let mut message = MessageBuilder::new();

    message.tag("school", Some(emoji::GRADUATION_CAP));

    write!(message, " New school added: ").unwrap();

    message.main_link(&event.name, &event.url);

    Some(Response {
        message,
        repo: None,
    })
}

fn handle_prolosite_impersonate(event: ImpersonatePayload) -> Option<Response> {
    let mut message = MessageBuilder::new();

    message.tag("impersonate", Some(emoji::POLICE_CAR_LIGHT));

    write!(&mut message, " ").unwrap();
    message.link(&event.hijacker.username, &event.hijacker.url);
    write!(&mut message, " {}ed impersonation of ", event.event).unwrap();
    message.main_link(&event.hijacked.username, &event.hijacked.url);

    Some(Response {
        message,
        repo: None,
    })
}

#[cfg(test)]
mod tests {
    use url::Url;

    use crate::webhooks::prolosite::{Exception, Request, User};

    use super::*;

    #[test]
    fn test_handle_prolosite_error() {
        let event = DjangoErrorPayload {
            request: Request {
                user: Some("prololo".to_string()),
                method: "GET".to_string(),
                path: "/some/route".into(),
            },
            exception: Exception {
                value: "ExampleException".to_string(),
                trace: vec![],
            },
        };

        let response = handle_prolosite_error(event).expect("should have a response");
        let message = response.message;

        assert_eq!(
            message.plain,
            "[🔥 django crash] (prololo) GET /some/route: ExampleException"
        );
        assert_eq!(
            message.html,
            "<b>[🔥 django crash]</b> (prololo) GET <code>/some/route</code>: <code>ExampleException</code>"
        );
    }
    #[test]
    fn test_handle_prolosite_impersonate() {
        let event = ImpersonatePayload {
            event: "start".to_string(),
            hijacker: User {
                username: "leo".to_string(),
                url: Url::parse("https://prologin.org/user/39194/profile").unwrap(),
            },
            hijacked: User {
                username: "prologin".to_string(),
                url: Url::parse("https://prologin.org/user/1/profile").unwrap(),
            },
        };

        let response = handle_prolosite_impersonate(event).expect("should have a response");
        let message = response.message;

        assert!(message.url.is_some());

        assert_eq!(
            message.plain,
            "[🚨 impersonate] leo started impersonation of prologin"
        );
        assert_eq!(
            message.html,
            r#"<b>[🚨 impersonate]</b> <a href="https://prologin.org/user/39194/profile">leo</a> started impersonation of <a href="https://prologin.org/user/1/profile">prologin</a>"#
        );
    }
}
