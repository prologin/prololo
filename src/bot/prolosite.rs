use std::fmt::Write;

use crate::{
    bot::{message_builder::MessageBuilder, Response},
    webhooks::{prolosite::DjangoErrorPayload, ProloSiteEvent},
};

pub(crate) fn handle_prolosite_event(event: ProloSiteEvent) -> anyhow::Result<Option<Response>> {
    let response = match event {
        ProloSiteEvent::Error(event) => handle_prolosite_error(event),
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

    // TODO: parse trace and show fancier exceptions
    let exception = &event.exception.value;
    write!(message, ": {}", exception).unwrap();

    Some(Response {
        message,
        repo: None,
    })
}
