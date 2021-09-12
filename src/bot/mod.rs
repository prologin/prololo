use std::{
    fs::File,
    io::{BufReader, BufWriter},
};

use matrix_sdk::{
    room::Room,
    ruma::events::{room::member::MemberEventContent, StrippedStateEvent},
    Client, ClientConfig, Session, SyncSettings,
};
use tracing::{debug, info};

use crate::config::ProloloConfig;

mod handlers;
use handlers::autojoin::autojoin_authorized_rooms;

pub struct Prololo {
    client: Client,
    config: ProloloConfig,
}

impl Prololo {
    /// Creates a new [`Prololo`] bot and builds a [`matrix_sdk::Client`] using the provided
    /// [`ProloloConfig`].
    ///
    /// The [`Client`] is only initialized, not ready to be used yet.
    pub fn new(config: ProloloConfig) -> anyhow::Result<Self> {
        let client_config = ClientConfig::new().store_path(config.matrix_state_dir.join("store"));
        let client = Client::new_with_config(config.matrix_homeserver.clone(), client_config)?;

        Ok(Self { client, config })
    }

    /// Loads session information from file, or creates it if no previous session is found.
    ///
    /// The bot is ready to run once this function has been called.
    pub async fn init(&self) -> anyhow::Result<()> {
        self.load_or_init_session().await?;

        let authorized_rooms = vec![self.config.matrix_room_id.clone()];

        self.client
            .register_event_handler({
                move |ev: StrippedStateEvent<MemberEventContent>, client: Client, room: Room| {
                    let authorized_rooms = authorized_rooms.clone();
                    debug!("handler!!!second");
                    async move { autojoin_authorized_rooms(ev, client, room, authorized_rooms).await }
                }
            })
            .await;

        Ok(())
    }

    /// Start listening to Matrix events.
    ///
    /// [`Prololo::init`] **must** be called before this function, otherwise the [`Client`] isn't
    /// logged in.
    pub async fn run(&self) {
        debug!("running...");
        self.client.sync(SyncSettings::default()).await
    }

    /// This loads the session information from an existing file, and tries to login with it. If no such
    /// file is found, then login using username and password, and save the new session information on
    /// disk.
    async fn load_or_init_session(&self) -> anyhow::Result<()> {
        let session_file = self.config.matrix_state_dir.join("session.yaml");

        if session_file.is_file() {
            let reader = BufReader::new(File::open(session_file)?);
            let session: Session = serde_yaml::from_reader(reader)?;

            self.client.restore_login(session.clone()).await?;
            info!("Reused session: {}, {}", session.user_id, session.device_id);
        } else {
            let response = self
                .client
                .login(
                    &self.config.matrix_username,
                    &self.config.matrix_password,
                    None,
                    Some("autojoin bot"),
                )
                .await?;

            info!("logged in as {}", self.config.matrix_username);

            let session = Session {
                access_token: response.access_token,
                user_id: response.user_id,
                device_id: response.device_id,
            };

            let writer = BufWriter::new(File::create(session_file)?);
            serde_yaml::to_writer(writer, &session)?;
        }

        Ok(())
    }
}
