use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::PathBuf,
};

use anyhow::Context;
use matrix_sdk::{
    room::Room,
    ruma::events::{
        room::member::MemberEventContent, room::message::MessageEventContent,
        AnyMessageEventContent, StrippedStateEvent,
    },
    Client, ClientConfig, Session, SyncSettings,
};
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, info, warn};

use crate::{
    config::ProloloConfig,
    webhooks::{github::RefType, Event, GitHubEvent},
};

mod handlers;
use handlers::autojoin_authorized_rooms;

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

        let authorized_rooms = vec![self.config.matrix_room_id.clone()];

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

            Self::handle_event(event, &client, &config).await
        }
    }

    async fn handle_event(event: Event, client: &Client, config: &ProloloConfig) {
        match event {
            Event::GitHub(event) => Self::handle_github_event(event, client, config).await,
        }
    }

    const SEPARATOR: &'static str = "⋅";

    async fn handle_github_event(event: GitHubEvent, client: &Client, config: &ProloloConfig) {
        let message = match event {
            GitHubEvent::Create(event) => match event.ref_type {
                RefType::Branch => return, // new branches are handled by the Push event
                RefType::Tag => {
                    format!(
                        "[{}] {} created tag {} {} {}",
                        event.repository.name,
                        event.sender.login,
                        event.r#ref,
                        Self::SEPARATOR,
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
