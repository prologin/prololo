use matrix_sdk::{
    ruma::events::{room::message::MessageEventContent, AnyMessageEventContent},
    Client,
};
use tracing::warn;

use crate::{
    config::ProloloConfig,
    webhooks::{github::RefType, GitHubEvent},
};

const SEPARATOR: &'static str = "⋅";

pub async fn handle_github_event(event: GitHubEvent, client: &Client, config: &ProloloConfig) {
    let message = match event {
        GitHubEvent::Create(event) => match event.ref_type {
            RefType::Branch => return, // new branches are handled by the Push event
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

    let room = match client.get_joined_room(&config.matrix_room_id) {
        Some(room) => room,
        None => {
            warn!(
                "room {} isn't joined yet, can't send message",
                config.matrix_room_id
            );
            return;
        }
    };

    if let Err(e) = room.send(message, None).await {
        warn!("encountered error while sending message: {}", e);
    }
}
