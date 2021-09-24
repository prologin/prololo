use std::fmt::Write;

use crate::{
    bot::Response,
    webhooks::{
        github::{
            CreateEvent, IssueCommentEvent, IssuesEvent, PullRequestEvent,
            PullRequestReviewCommentEvent, PullRequestReviewEvent, RefType,
        },
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
        GitHubEvent::PullRequest(event) => handle_pull_request(event),
        GitHubEvent::PullRequestReview(event) => handle_pull_request_review(event),
        GitHubEvent::PullRequestReviewComment(event) => handle_pull_request_review_comment(event),
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

    // Comments left on PRs are considered as issue comments as well
    let issue_or_pr = match issue.pull_request {
        Some(_) => "PR",
        None => "issue",
    };

    let mut message = format!("[{}] {}", event.repository.name, event.sender.login);

    match action.as_str() {
        "created" => write!(
            message,
            " commented on {} {}: {}",
            issue_or_pr, issue, comment.body
        )
        .unwrap(),

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

fn handle_pull_request(event: PullRequestEvent) -> Option<Response> {
    let action = event.action;
    let pr = event.pull_request;

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
            write!(message, " to {}", pr).unwrap();
        }

        "review_requested" => {
            let reviewers = pr
                .requested_reviewers
                .iter()
                .map(|user| user.login.as_str())
                .collect::<Vec<&str>>()
                .join(", ");

            write!(message, " requested {} to review {}", reviewers, pr).unwrap();
        }

        // too verbose, don't log that
        "labeled" | "unlabeled" | "review_requested_removed" => return None,

        "opened" | "edited" | "reopened" => {
            let base = &pr.base.r#ref;
            let head = &pr.head.r#ref;
            write!(message, " {} {} ({}...{})", action, pr, base, head).unwrap();
        }

        "closed" => {
            let decision = if pr
                .merged
                .expect("PR should always have a merged field in this case")
            {
                "merged"
            } else {
                "closed"
            };
            write!(message, " {} {}", decision, pr).unwrap();
        }

        _ => return None, // FIXME log error
    }

    write!(message, " {} {}", SEPARATOR, pr.html_url).unwrap();

    Some(Response {
        message,
        repo: Some(event.repository.full_name),
    })
}

fn handle_pull_request_review(event: PullRequestReviewEvent) -> Option<Response> {
    let action = event.action;
    let review = event.review;
    let reviewer = review.user.login;
    let pr = event.pull_request;

    let state = review.state;

    let decision = match state.to_lowercase().as_str() {
        "approved" => "approved",
        "changes_requested" => "requested changes on",
        // FIXME: couldn't find the value of state for comment reviews, find out what it is and make
        //        sure there's a proper error in other cases
        _ => "commented on",
    };

    let mut message = format!("[{}] {}", event.repository.name, event.sender.login);

    match action.as_str() {
        "submitted" => write!(message, " {} {}", decision, pr).unwrap(),

        // ignored, too verbose
        "edited" => return None,

        "dismissed" => {
            write!(message, " dismissed ").unwrap();

            if event.sender.login == reviewer {
                write!(message, "their").unwrap();
            } else {
                write!(message, "{}'s", reviewer).unwrap();
            };

            write!(message, " review for {} (they {} the PR)", pr, decision).unwrap();
        }

        _ => return None, // FIXME log error
    }

    write!(message, " {} {}", SEPARATOR, review.html_url).unwrap();

    Some(Response {
        message,
        repo: Some(event.repository.full_name),
    })
}

fn handle_pull_request_review_comment(event: PullRequestReviewCommentEvent) -> Option<Response> {
    let action = event.action;
    let comment = event.comment;
    let pr = event.pull_request;

    if comment.pull_request_review_id.is_some() {
        // Inline code comment is linked to a PR review, no need to display a message for every
        // comment in that review.
        //
        // Global review event will be handled by the `pull_request_review` event.
        return None;
    }

    let mut message = format!("[{}] {}", event.repository.name, event.sender.login);

    match action.as_str() {
        "created" => write!(message, " commented on {} {}", pr, comment.location()).unwrap(),

        // ignored, too verbose
        "edited" | "deleted" => return None,

        _ => return None, // FIXME log error
    }

    write!(message, " {} {}", SEPARATOR, comment.html_url).unwrap();

    Some(Response {
        message,
        repo: Some(event.repository.full_name),
    })
}
