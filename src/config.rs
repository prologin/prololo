use std::path::PathBuf;

use matrix_sdk::ruma::RoomId;
use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct ProloloConfig {
    /// The URL for the homeserver we should connect to
    pub matrix_homeserver: Url,
    /// The bot's account username
    pub matrix_username: String,
    /// The bot's account password
    pub matrix_password: String,
    /// Path to a directory where the bot will store Matrix state and current session information.
    pub matrix_state_dir: PathBuf,
    /// ID of the Matrix room where the bot should post messages. The bot will only accept
    /// invitations to this room.
    pub matrix_room_id: RoomId,
}
