//! PermissionRequest hooks the node holds open while a person decides on another device
//! (EXP-0001). Claude Code draws its dialog at the same time and queues any keystroke typed
//! there until the hook returns, so every hold ends one of four ways: a resolution from the
//! hub, a keystroke at a wrapped terminal, the fixed hold for an unwrapped session running
//! out, or the session going away. All but the first answer `{}`, which leaves the dialog to
//! the terminal. AskUserQuestion is held the same way and answered through `updatedInput`
//! (EXP-0004).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use rc2s2_proto::event::v1::{PendingKind, PendingRequest, ReleaseReason, Resolution, resolution};
use serde_json::{Value, json};
use tokio::sync::{Mutex, oneshot};

use crate::outbox::{Outbox, now_ms};

pub const QUESTION_TOOL: &str = "AskUserQuestion";

pub struct Pending {
    outbox: Outbox,
    held: Mutex<HashMap<String, Held>>,
}

struct Held {
    request: PendingRequest,
    tool_input: Value,
    release: oneshot::Sender<(Value, ReleaseReason)>,
}

impl Pending {
    pub fn new(outbox: Outbox) -> Arc<Self> {
        Arc::new(Self {
            outbox,
            held: Mutex::new(HashMap::new()),
        })
    }

    /// Holds a PermissionRequest hook input for at most `hold` and returns the hook's response.
    pub async fn hold(self: &Arc<Self>, input: &Value, hold: Duration) -> Value {
        let tool_name = input["tool_name"].as_str().unwrap_or_default();
        let request = PendingRequest {
            id: uuid::Uuid::new_v4().simple().to_string(),
            session_id: input["session_id"].as_str().unwrap_or_default().to_string(),
            kind: if tool_name == QUESTION_TOOL {
                PendingKind::Question
            } else {
                PendingKind::Permission
            }
            .into(),
            tool_name: tool_name.to_string(),
            tool_input_json: input["tool_input"].to_string(),
            created_ms: now_ms(),
        };
        let id = request.id.clone();
        let (release, released) = oneshot::channel();
        self.held.lock().await.insert(
            id.clone(),
            Held {
                request: request.clone(),
                tool_input: input["tool_input"].clone(),
                release,
            },
        );
        self.outbox.pending(request).await;

        // If Claude Code gives up on the hook (it exited, or its own timeout fired), this
        // future is dropped and the guard closes the request.
        let mut guard = Guard {
            pending: self.clone(),
            id: Some(id.clone()),
        };
        let (response, reason) = tokio::select! {
            released = released => released.unwrap_or((json!({}), ReleaseReason::SessionEnded)),
            _ = tokio::time::sleep(hold) => {
                self.held.lock().await.remove(&id);
                (json!({}), ReleaseReason::Timeout)
            }
        };
        guard.id = None;
        self.outbox.pending_closed(&id, reason).await;
        response
    }

    /// Applies a resolution from the hub. An error says why it was refused and leaves the
    /// request held.
    pub async fn resolve(&self, resolution: &Resolution) -> Result<(), String> {
        let mut held = self.held.lock().await;
        let Some(h) = held.get(&resolution.pending_id) else {
            return Err("no such pending request on this node; it may have been released".into());
        };
        let decision = match &resolution.decision {
            Some(resolution::Decision::Allow(_)) => json!({"behavior": "allow"}),
            Some(resolution::Decision::Deny(deny)) => {
                json!({"behavior": "deny", "message": deny.message})
            }
            Some(resolution::Decision::Answers(answers)) => {
                if h.request.kind() != PendingKind::Question {
                    return Err("answers apply only to a question".into());
                }
                let updated = answered_input(&h.tool_input, answers)?;
                json!({"behavior": "allow", "updatedInput": updated})
            }
            None => return Err("the resolution has no decision".into()),
        };
        let h = held.remove(&resolution.pending_id).expect("checked above");
        let response = json!({
            "hookSpecificOutput": {"hookEventName": "PermissionRequest", "decision": decision}
        });
        let _ = h.release.send((response, ReleaseReason::Resolved));
        Ok(())
    }

    /// Releases every request held for a session, leaving the decision to its terminal.
    pub async fn release_session(&self, session_id: &str, reason: ReleaseReason) {
        let mut held = self.held.lock().await;
        let ids: Vec<String> = held
            .iter()
            .filter(|(_, h)| h.request.session_id == session_id)
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            if let Some(h) = held.remove(&id) {
                let _ = h.release.send((json!({}), reason));
            }
        }
    }

    /// The requests held now, for the registration after a (re)connect.
    pub async fn snapshot(&self) -> Vec<PendingRequest> {
        self.held
            .lock()
            .await
            .values()
            .map(|h| h.request.clone())
            .collect()
    }
}

struct Guard {
    pending: Arc<Pending>,
    id: Option<String>,
}

impl Drop for Guard {
    fn drop(&mut self) {
        if let Some(id) = self.id.take() {
            let pending = self.pending.clone();
            tokio::spawn(async move {
                pending.held.lock().await.remove(&id);
                pending
                    .outbox
                    .pending_closed(&id, ReleaseReason::SessionEnded)
                    .await;
            });
        }
    }
}

/// The AskUserQuestion input with an `answers` map added, checked the way Claude Code checks
/// it (EXP-0004): keys are the exact question texts and every question is answered; a
/// single-select or free-text answer is one string, a multi-select answer an array of at
/// most the options plus one free-text entry.
fn answered_input(
    tool_input: &Value,
    answers: &rc2s2_proto::event::v1::Answers,
) -> Result<Value, String> {
    let questions = tool_input["questions"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut map = serde_json::Map::new();
    for q in &questions {
        let text = q["question"].as_str().unwrap_or_default();
        let Some(answer) = answers.answers.get(text) else {
            return Err(format!("no answer for the question {text:?}"));
        };
        let value = if q["multiSelect"].as_bool().unwrap_or(false) {
            let options = q["options"].as_array().map_or(0, Vec::len);
            if answer.values.is_empty() || answer.values.len() > options + 1 {
                return Err(format!(
                    "{text:?} takes between 1 and {} answers",
                    options + 1
                ));
            }
            json!(answer.values)
        } else {
            let [one] = answer.values.as_slice() else {
                return Err(format!("{text:?} takes exactly one answer"));
            };
            json!(one)
        };
        map.insert(text.to_string(), value);
    }
    if let Some(extra) = answers
        .answers
        .keys()
        .find(|k| !questions.iter().any(|q| q["question"] == k.as_str()))
    {
        return Err(format!("{extra:?} is not one of the questions"));
    }
    let mut updated = tool_input.clone();
    updated["answers"] = Value::Object(map);
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc2s2_proto::event::v1::{AnswerValue, Answers, Deny};
    use rc2s2_proto::node::v1::{NodeMessage, node_message::Body};

    fn permission(session: &str) -> Value {
        json!({"hook_event_name": "PermissionRequest", "session_id": session,
               "tool_name": "Bash", "tool_input": {"command": "rm -rf build"}})
    }

    fn question() -> Value {
        json!({"hook_event_name": "PermissionRequest", "session_id": "s1",
               "tool_name": QUESTION_TOOL, "tool_input": {"questions": [
                   {"question": "Which db?", "header": "DB", "multiSelect": false,
                    "options": [{"label": "sqlite"}, {"label": "postgres"}]},
                   {"question": "Which targets?", "header": "Targets", "multiSelect": true,
                    "options": [{"label": "darwin"}, {"label": "linux"}]}]}})
    }

    /// Waits for the request the node reported to the hub.
    async fn reported(rx: &mut tokio::sync::mpsc::Receiver<NodeMessage>) -> PendingRequest {
        loop {
            if let Some(Body::Pending(p)) = rx.recv().await.unwrap().body {
                return p;
            }
        }
    }

    async fn closed(rx: &mut tokio::sync::mpsc::Receiver<NodeMessage>) -> ReleaseReason {
        loop {
            if let Some(Body::PendingClosed(c)) = rx.recv().await.unwrap().body {
                return c.reason();
            }
        }
    }

    fn resolution(id: &str, decision: resolution::Decision) -> Resolution {
        Resolution {
            pending_id: id.to_string(),
            decision: Some(decision),
        }
    }

    fn answers(pairs: &[(&str, &[&str])]) -> resolution::Decision {
        resolution::Decision::Answers(Answers {
            answers: pairs
                .iter()
                .map(|(q, v)| {
                    let values = v.iter().map(|s| s.to_string()).collect();
                    (q.to_string(), AnswerValue { values })
                })
                .collect(),
        })
    }

    #[tokio::test]
    async fn a_resolution_answers_the_hook() {
        let (outbox, mut rx) = Outbox::new(16);
        let pending = Pending::new(outbox);
        let hold = {
            let pending = pending.clone();
            tokio::spawn(async move {
                pending
                    .hold(&permission("s1"), Duration::from_secs(10))
                    .await
            })
        };
        let p = reported(&mut rx).await;
        assert_eq!(p.kind(), PendingKind::Permission);
        assert_eq!(pending.snapshot().await.len(), 1);
        let deny = Deny {
            message: "not now".into(),
        };
        pending
            .resolve(&resolution(&p.id, resolution::Decision::Deny(deny)))
            .await
            .unwrap();
        let response = hold.await.unwrap();
        assert_eq!(
            response["hookSpecificOutput"]["decision"],
            json!({"behavior": "deny", "message": "not now"})
        );
        assert_eq!(closed(&mut rx).await, ReleaseReason::Resolved);
        assert!(pending.snapshot().await.is_empty());
        let again = resolution(&p.id, resolution::Decision::Allow(Default::default()));
        assert!(pending.resolve(&again).await.is_err());
    }

    #[tokio::test]
    async fn an_unanswered_hold_falls_through_to_the_terminal() {
        let (outbox, mut rx) = Outbox::new(16);
        let pending = Pending::new(outbox);
        let response = pending
            .hold(&permission("s1"), Duration::from_millis(50))
            .await;
        assert_eq!(response, json!({}));
        reported(&mut rx).await;
        assert_eq!(closed(&mut rx).await, ReleaseReason::Timeout);
        assert!(pending.snapshot().await.is_empty());
    }

    #[tokio::test]
    async fn releasing_a_session_and_dropping_a_hold_close_it() {
        let (outbox, mut rx) = Outbox::new(16);
        let pending = Pending::new(outbox);
        let hold = {
            let pending = pending.clone();
            tokio::spawn(async move {
                pending
                    .hold(&permission("s1"), Duration::from_secs(10))
                    .await
            })
        };
        reported(&mut rx).await;
        pending
            .release_session("s1", ReleaseReason::Keystroke)
            .await;
        assert_eq!(hold.await.unwrap(), json!({}));
        assert_eq!(closed(&mut rx).await, ReleaseReason::Keystroke);

        // Claude Code giving up on the hook drops the handler's future.
        let hold = {
            let pending = pending.clone();
            tokio::spawn(async move {
                pending
                    .hold(&permission("s2"), Duration::from_secs(10))
                    .await
            })
        };
        reported(&mut rx).await;
        hold.abort();
        assert_eq!(closed(&mut rx).await, ReleaseReason::SessionEnded);
        assert!(pending.snapshot().await.is_empty());
    }

    #[tokio::test]
    async fn answers_become_updated_input() {
        let (outbox, mut rx) = Outbox::new(16);
        let pending = Pending::new(outbox);
        let hold = {
            let pending = pending.clone();
            tokio::spawn(async move { pending.hold(&question(), Duration::from_secs(10)).await })
        };
        let p = reported(&mut rx).await;
        assert_eq!(p.kind(), PendingKind::Question);
        let bad = [
            answers(&[("Which db?", &["sqlite"])]),
            answers(&[
                ("Which db?", &["sqlite", "postgres"]),
                ("Which targets?", &["linux"]),
            ]),
            answers(&[
                ("Which db?", &["a"]),
                ("Which targets?", &["a", "b", "c", "d"]),
            ]),
            answers(&[
                ("Which db?", &["a"]),
                ("Which targets?", &["a"]),
                ("Other?", &["a"]),
            ]),
        ];
        for decision in bad {
            assert!(pending.resolve(&resolution(&p.id, decision)).await.is_err());
        }
        let good = answers(&[
            ("Which db?", &["a key-value store"]),
            ("Which targets?", &["darwin", "linux"]),
        ]);
        pending.resolve(&resolution(&p.id, good)).await.unwrap();
        let decision = &hold.await.unwrap()["hookSpecificOutput"]["decision"];
        assert_eq!(decision["behavior"], "allow");
        assert_eq!(
            decision["updatedInput"]["answers"],
            json!({"Which db?": "a key-value store", "Which targets?": ["darwin", "linux"]})
        );
        assert_eq!(decision["updatedInput"]["questions"][0]["header"], "DB");
    }
}
