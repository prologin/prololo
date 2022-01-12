use std::fmt::Write;

use tracing::trace;

use crate::{
    bot::{message_builder::MessageBuilder, Response},
    webhooks::GenericEvent,
};

pub(crate) fn handle_generic_event(event: GenericEvent) -> anyhow::Result<Option<Response>> {
    trace!("handling generic event");
    let GenericEvent(event) = event;

    let mut message = MessageBuilder::new();

    if let Some(tag) = event.tag {
        message.tag(&tag, None);
        write!(message, " ").unwrap();
    }

    write!(message, "{}", event.message).unwrap();

    if let Some(url) = event.url {
        write!(message, " ({})", url).unwrap();
    }

    Ok(Some(Response {
        message,
        repo: None,
    }))
}

#[cfg(test)]
mod tests {
    use url::Url;

    use crate::webhooks::generic::GenericPayload;

    use super::*;

    #[test]
    fn test_handle_generic_event() {
        let event = GenericEvent(GenericPayload {
            message: "Hello World!".to_string(),
            tag: Some("generic".to_string()),
            url: Some(Url::parse("https://prologin.org/").unwrap()),
        });

        let response = handle_generic_event(event)
            .expect("should have a response")
            .unwrap();
        let message = response.message;

        assert_eq!(
            message.plain,
            "[generic] Hello World! (https://prologin.org/)"
        );

        assert_eq!(
            message.html,
            "<b>[generic]</b> Hello World! (https://prologin.org/)"
        );
    }
}
