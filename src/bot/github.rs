use matrix_sdk::ruma::events::{room::message::MessageEventContent, AnyMessageEventContent};

use crate::webhooks::{github::RefType, GitHubEvent};

const SEPARATOR: &str = "⋅";

pub fn handle_github_event(
    event: GitHubEvent,
) -> anyhow::Result<Option<AnyMessageEventContent>> {
    let message = match event {
        GitHubEvent::Create(event) => match event.ref_type {
            RefType::Branch => return Ok(None), // new branches are handled by the Push event
            RefType::Tag => {
                format!(
                    "[{}] {} created tag {} {} {}",
                    event.repository.name,
                    event.sender.login,
                    event.r#ref,
                    SEPARATOR,
                    event.repository.ref_url(&event.r#ref)
                )
            }
        },
        GitHubEvent::Issues => todo!(),
        GitHubEvent::IssueComment => todo!(),
        GitHubEvent::Push => todo!(),
    };

    let message = AnyMessageEventContent::RoomMessage(MessageEventContent::text_plain(message));

    Ok(Some(message))
}
