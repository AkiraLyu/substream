use std::time::Instant;

use substream_protocol::{
    ServerMessage,
    display::{DisplayState, DisplayStatus},
};
use tokio::sync::watch;

#[derive(Clone, Default)]
pub(crate) struct DisplayRecord {
    state: DisplayState,
    caption_at: Option<Instant>,
}

impl DisplayRecord {
    pub fn snapshot(&self) -> DisplayState {
        let mut state = self.state.clone();
        state.caption_age_ms = self
            .caption_at
            .map(|at| at.elapsed().as_millis().try_into().unwrap_or(u64::MAX));
        state
    }
}

#[derive(Clone)]
pub(crate) struct DisplayHub(watch::Sender<DisplayRecord>);

impl Default for DisplayHub {
    fn default() -> Self {
        Self(watch::channel(DisplayRecord::default()).0)
    }
}

impl DisplayHub {
    pub fn subscribe(&self) -> watch::Receiver<DisplayRecord> {
        self.0.subscribe()
    }

    pub fn start(&self) -> DisplaySession {
        let mut id = 0;
        self.0.send_modify(|record| {
            id = record.state.session_id.wrapping_add(1);
            *record = DisplayRecord {
                state: DisplayState {
                    session_id: id,
                    status: DisplayStatus::Loading,
                    ..DisplayState::default()
                },
                caption_at: None,
            };
        });
        DisplaySession {
            hub: self.clone(),
            id,
        }
    }
}

pub(crate) struct DisplaySession {
    hub: DisplayHub,
    id: u64,
}

impl DisplaySession {
    pub fn publish(&self, event: &ServerMessage) {
        self.hub.0.send_if_modified(|record| {
            if record.state.session_id != self.id {
                return false;
            }
            match event {
                ServerMessage::Ready { backend, .. } => {
                    record.state.status = DisplayStatus::Listening;
                    record.state.backend = Some(backend.clone());
                }
                ServerMessage::Caption { caption } => {
                    record.state.caption = Some(caption.clone());
                    record.caption_at = Some(Instant::now());
                }
                ServerMessage::Finished { .. } => record.state.status = DisplayStatus::Finished,
                ServerMessage::Error { message, .. } => {
                    record.state.status = DisplayStatus::Error;
                    record.state.message = Some(message.clone());
                    record.state.caption = None;
                    record.caption_at = None;
                }
            }
            true
        });
    }
}

impl Drop for DisplaySession {
    fn drop(&mut self) {
        self.hub.0.send_if_modified(|record| {
            if record.state.session_id != self.id
                || !matches!(
                    record.state.status,
                    DisplayStatus::Loading | DisplayStatus::Listening
                )
            {
                return false;
            }
            *record = DisplayRecord {
                state: DisplayState {
                    session_id: self.id,
                    ..DisplayState::default()
                },
                caption_at: None,
            };
            true
        });
    }
}
