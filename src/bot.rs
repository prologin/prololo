use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::PathBuf,
};

use anyhow::{anyhow, Context};
use matrix_sdk::{
    room::Room,
    ruma::{
        events::{room::member::MemberEventContent, AnyMessageEventContent, StrippedStateEvent},
        RoomId,
    },
    Client, ClientConfig, Session, SyncSettings,
};
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, info, trace, warn};

use crate::{config::ProloloConfig, webhooks::Event};

mod github;
use github::handle_github_event;

mod handlers;
use handlers::autojoin_authorized_rooms;

mod message_builder;
use message_builder::MessageBuilder;

pub(crate) mod utils;

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
        self.load_or_init_session()
            .await
            .context("couldn't init session for matrix bot")?;

        let authorized_rooms: Vec<RoomId> = self
            .config
            .matrix_rooms
            .values()
            .map(|room| room.id.clone())
            .collect();

        self.client
            .register_event_handler({
                move |ev: StrippedStateEvent<MemberEventContent>, client: Client, room: Room| {
                    let authorized_rooms = authorized_rooms.clone();
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
    pub async fn run(&self, events: UnboundedReceiver<Event>) {
        debug!("running...");

        let client = self.client.clone();
        let config = self.config.clone();
        tokio::task::spawn(async move { Self::receive_events(events, client, config).await });

        self.client.sync(SyncSettings::default()).await
    }

    async fn receive_events(
        mut events: UnboundedReceiver<Event>,
        client: Client,
        config: ProloloConfig,
    ) {
        loop {
            let event = match events.recv().await {
                Some(event) => event,
                None => {
                    info!("all channel senders were dropped, exiting receive loop");
                    break;
                }
            };
            debug!("received event: {:?}", event);

            if let Err(e) = Self::handle_event(event, &client, &config).await {
                warn!("encountered error while handling event: {}", e);
            }
        }
    }

    async fn handle_event(
        event: Event,
        client: &Client,
        config: &ProloloConfig,
    ) -> anyhow::Result<()> {
        let response = match event {
            Event::GitHub(event) => handle_github_event(event)?,
        };

        let Response { message, repo } = match response {
            Some(response) => response,
            // event doesn't need a message from the bot
            None => {
                trace!("event didn't need to be announced");
                return Ok(());
            }
        };

        let room = repo
            // get room id for current repo, or use default room
            .map_or_else(|| config.default_room(), |repo| config.find_room_for(repo))
            // find that joined room in the Matrix client
            .and_then(|room_id| {
                client.get_joined_room(room_id).ok_or_else(|| {
                    anyhow!(
                        "room with id {} isn't joined yet, can't send message",
                        room_id
                    )
                })
            })?;

        trace!(
            "sending message `{}` to room `{}`",
            message.plain,
            room.room_id()
        );
        let message = AnyMessageEventContent::RoomMessage(message.into());
        room.send(message, None).await?;

        Ok(())
    }

    /// This loads the session information from an existing file, and tries to login with it. If no such
    /// file is found, then login using username and password, and save the new session information on
    /// disk.
    async fn load_or_init_session(&self) -> anyhow::Result<()> {
        let session_file = PathBuf::from("matrix-session.yaml");

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

pub struct Response {
    pub message: MessageBuilder,
    pub repo: Option<String>,
}
