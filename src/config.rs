use std::{collections::HashMap, path::PathBuf};

use anyhow::anyhow;
use matrix_sdk::ruma::RoomId;
use regex::Regex;
use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize, Clone)]
pub struct ProloloConfig {
    /// The URL for the homeserver we should connect to
    pub matrix_homeserver: Url,
    /// The bot's account username
    pub matrix_username: String,
    /// The bot's account password
    pub matrix_password: String,
    /// Path to a directory where the bot will store Matrix state and current session information.
    pub matrix_state_dir: PathBuf,
    /// Matrix rooms that the bot should join. The bot will only accept invitations to these rooms.
    pub matrix_rooms: HashMap<String, RoomConfig>,
    /// Mappings from all repos matching a certain regex, to a specific Matrix room
    pub destinations: Vec<Destination>,
    #[serde(default)]
    /// Generic endpoints
    pub generic_endpoints: HashMap<String, GenericEndpoint>,
    /// Secret used to verify HMAC signature of GitHub webhooks
    pub github_secret: String,
    /// Secret token used in Authorization header for Prologin site hooks
    pub prolosite_secret: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RoomConfig {
    /// The room's ID in Matrix
    pub id: RoomId,
    /// The default room will receive all messages that didn't match any destination
    #[serde(default)]
    default: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Destination {
    /// The room name as used in [`ProloloConfig::matrix_rooms`]
    pub room: String,
    /// The regex used to match some repos to this destination
    #[serde(with = "serde_regex")]
    pub regex: Regex,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GenericEndpoint {
    /// The room name as used in [`ProloloConfig::matrix_rooms`]
    pub room: String,
    /// The secret used to authenticate requests to this endpoint
    pub secret: String,
}

impl ProloloConfig {
    pub fn find_room_for(&self, repo: String) -> anyhow::Result<&RoomId> {
        let matched = self
            .destinations
            .iter()
            .find(|dest| dest.regex.is_match(&repo));

        match matched {
            Some(dest) => self
                .matrix_rooms
                .get(&dest.room)
                .map(|room| &room.id)
                .ok_or_else(|| anyhow!("destination points to unknown room {}", dest.room)),
            None => self.default_room(),
        }
    }

    pub fn default_room(&self) -> anyhow::Result<&RoomId> {
        self.matrix_rooms
            .values()
            .find(|room| room.default)
            .map(|room| &room.id)
            .ok_or_else(|| anyhow!("no default room provided!"))
    }
}
