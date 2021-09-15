use matrix_sdk::ruma::events::{room::message::MessageEventContent, AnyMessageEventContent};

use crate::webhooks::{
    github::{CreateEvent, RefType},
    GitHubEvent,
};

const SEPARATOR: &str = "⋅";

pub fn handle_github_event(event: GitHubEvent) -> anyhow::Result<Option<AnyMessageEventContent>> {
    let message = match event {
        GitHubEvent::Create(event) => handle_create(event),
        GitHubEvent::Issues => todo!(),
        GitHubEvent::IssueComment => todo!(),
        GitHubEvent::Push => todo!(),
    };

    Ok(message.map(|m| AnyMessageEventContent::RoomMessage(MessageEventContent::text_plain(m))))
}

fn handle_create(event: CreateEvent) -> Option<String> {
    match event.ref_type {
        RefType::Branch => None,
        RefType::Tag => Some(format!(
            "[{}] {} created tag {} {} {}",
            event.repository.name,
            event.sender.login,
            event.r#ref,
            SEPARATOR,
            event.repository.ref_url(&event.r#ref)
        )),
    }
}
