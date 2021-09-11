use std::time::Duration;

use matrix_sdk::{
    room::Room,
    ruma::{
        events::{room::member::MemberEventContent, StrippedStateEvent},
        RoomId,
    },
    Client,
};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

pub async fn autojoin_authorized_rooms(
    room_member: StrippedStateEvent<MemberEventContent>,
    client: Client,
    room: Room,
    authorized_rooms: Vec<RoomId>,
) {
    if room_member.state_key != client.user_id().await.unwrap() {
        return;
    }

    if let Room::Invited(room) = room {
        let room_id = room.room_id();
        let room_name = room
            .display_name()
            .await
            .expect("couldn't get joined room name!");
        info!(
            "Received invitation for room `{}`: `{}`",
            room_id, room_name
        );

        if authorized_rooms.contains(room_id) {
            warn!(
                "Bot isn't authorized to join room `{}`, declining invitation",
                room_id
            );
            room.reject_invitation().await.unwrap();
            return;
        }

        debug!("Autojoining room {}", room.room_id());
        let mut delay = 2;

        while let Err(err) = room.accept_invitation().await {
            // retry autojoin due to synapse sending invites, before the
            // invited user can join for more information see
            // https://github.com/matrix-org/synapse/issues/4345
            warn!(
                "Failed to join room {} ({:?}), retrying in {}s",
                room.room_id(),
                err,
                delay
            );

            sleep(Duration::from_secs(delay)).await;
            delay *= 2;

            if delay > 3600 {
                error!("Can't join room {} ({:?})", room.room_id(), err);
                break;
            }
        }
        info!("Successfully joined room {}", room.room_id());
    }
}
