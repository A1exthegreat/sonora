//! Turns a shortcut into a line in the decisions log `sonora decisions` reads.

use std::time::{SystemTime, UNIX_EPOCH};

use cli::decisions::{self, Action, Decision, TrackRef};
use music::Track;

/// What a shortcut decided about the track that was playing.
pub enum Choice {
    /// A rating from 1 to 5, or 0 for no rating.
    Rate(u8),
    /// Whether the track should be saved to the library.
    Favorite(bool),
    /// The track is to be left out of the library, not played again.
    Discard,
}

/// Appends `choice` about `track` to the log. A track without an id cannot be found again, so
/// nothing is written for it. A failed write is logged and otherwise ignored: the shortcut still
/// did what it could, and the player must not come to a stop over a full disk.
pub fn record(choice: Choice, track: &Track, provider: Option<String>) {
    let Some(id) = track.id.clone() else {
        return;
    };
    let decision = Decision {
        at: unix_time(),
        action: match choice {
            Choice::Rate(stars) => Action::Rate { stars },
            Choice::Favorite(saved) => Action::Favorite { saved },
            Choice::Discard => Action::Discard,
        },
        track: TrackRef {
            id,
            title: track.name.clone(),
            artist: track.artists.clone(),
            album: track.album.clone(),
            provider,
        },
    };
    if let Err(error) = decisions::append(&decision) {
        log::warn!("decisions: cannot append the decision: {error}");
    }
}

fn unix_time() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() as i64)
}
