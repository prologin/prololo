use std::fmt::Write;

use tracing::trace;

use crate::{
    bot::{message_builder::MessageBuilder, utils::shorten_content_length, Response},
    webhooks::{
        prolosite::{DjangoErrorPayload, ForumPayload, NewSchoolPayload},
        ProloSiteEvent,
    },
};

pub(crate) fn handle_prolosite_event(event: ProloSiteEvent) -> anyhow::Result<Option<Response>> {
    trace!("handling prolosite event");
    let response = match event {
        ProloSiteEvent::Error(event) => handle_prolosite_error(event),
        ProloSiteEvent::Forum(event) => handle_prolosite_forum(event),
        ProloSiteEvent::NewSchool(event) => handle_prolosite_new_school(event),
    };

    Ok(response)
}

fn handle_prolosite_error(event: DjangoErrorPayload) -> Option<Response> {
    let mut message = MessageBuilder::new();

    message.tag("django crash");

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

    message.tag("forum");

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

    message.tag("school");

    write!(message, " New school added: ").unwrap();

    message.main_link(&event.name, &event.url);

    Some(Response {
        message,
        repo: None,
    })
}

#[cfg(test)]
mod tests {
    use crate::webhooks::prolosite::{Exception, Request};

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
            "[django crash] (prololo) GET /some/route: ExampleException"
        );
        assert_eq!(
            message.html,
            "<b>[django crash]</b> (prololo) GET <code>/some/route</code>: <code>ExampleException</code>"
        );
    }
}
