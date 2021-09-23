use std::fmt::Write;

use crate::{
    bot::Response,
    webhooks::{
        github::{CreateEvent, IssueCommentEvent, IssuesEvent, RefType},
        GitHubEvent,
    },
};

const SEPARATOR: &str = "⋅";

pub fn handle_github_event(event: GitHubEvent) -> anyhow::Result<Option<Response>> {
    let response = match event {
        GitHubEvent::Create(event) => handle_create(event),
        GitHubEvent::Issues(event) => handle_issues(event),
        GitHubEvent::IssueComment(event) => handle_issue_comment(event),
        GitHubEvent::Push => todo!(),
    };

    Ok(response)
}

fn handle_create(event: CreateEvent) -> Option<Response> {
    let message = match event.ref_type {
        RefType::Branch => return None,
        RefType::Tag => format!(
            "[{}] {} created tag {} {} {}",
            event.repository.name,
            event.sender.login,
            event.r#ref,
            SEPARATOR,
            event.repository.ref_url(&event.r#ref)
        ),
    };

    Some(Response {
        message,
        repo: Some(event.repository.full_name),
    })
}

fn handle_issues(event: IssuesEvent) -> Option<Response> {
    let action = event.action;
    let issue = event.issue;

    let mut message = format!("[{}] {}", event.repository.name, event.sender.login);

    match action.as_str() {
        "assigned" | "unassigned" => {
            let assignee = event
                .assignee
                .expect("assigned action should always have an assignee");
            let sender = event.sender;
            if assignee.id == sender.id {
                write!(message, " self-{}", action).unwrap();
            } else {
                write!(message, " {} {}", action, assignee.login).unwrap();
            }
            write!(message, " to {}", issue).unwrap();
        }

        // too verbose, don't log that
        "labeled" | "unlabeled" => return None,

        "opened" | "deleted" | "pinned" | "unpinned" | "reopened" | "closed" | "locked"
        | "unlocked" | "transferred" => write!(message, " {} issue {}", action, issue).unwrap(),

        "edited" => {
            let changes = event
                .changes
                .expect("edited issue without changes shouldn't happen");

            write!(message, " edited").unwrap();
            if changes.title.is_some() {
                write!(message, " title").unwrap();
            }
            if changes.body.is_some() {
                if changes.title.is_some() {
                    write!(message, ",").unwrap();
                }
                write!(message, " body").unwrap();
            }
            write!(message, " of issue {}", issue).unwrap();
        }

        "milestoned" => {
            let milestone = issue
                .milestone
                .as_ref()
                .expect("milestoned issue should have a milestone");
            write!(message, " added milestone {} to {}", milestone.title, issue).unwrap();
        }

        // https://github.com/isaacs/github/issues/880
        "demilestoned" => write!(message, " removed the milestone from {}", issue).unwrap(),

        _ => return None, // FIXME log error
    }

    write!(message, " {} {}", SEPARATOR, issue.html_url).unwrap();

    Some(Response {
        message,
        repo: Some(event.repository.full_name),
    })
}

fn handle_issue_comment(event: IssueCommentEvent) -> Option<Response> {
    let action = event.action;
    let comment = event.comment;
    let issue = event.issue;

    let mut message = format!("[{}] {}", event.repository.name, event.sender.login);

    match action.as_str() {
        "created" => write!(message, " commented on issue {}: {}", issue, comment.body).unwrap(),

        // too verbose, don't log that
        "edited" | "deleted" => return None,

        _ => return None, // FIXME log error
    }

    write!(message, " {} {}", SEPARATOR, comment.html_url).unwrap();

    Some(Response {
        message,
        repo: Some(event.repository.full_name),
    })
}
