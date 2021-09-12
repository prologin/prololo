use std::sync::{mpsc::SyncSender};

mod github;
pub use github::{github_webhook, GitHubEvent};

pub struct EventSender(pub SyncSender<Event>);

#[derive(Debug)]
pub enum Event {
    GitHub(GitHubEvent),
}
