//! The decisions the rating shortcuts make about the track that plays, kept as one JSON object
//! per line under the state folder. The app appends them; `sonora decisions` reads them without
//! needing a running app, so a script or an agent can follow what the listener decided.

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The file under `<state>/sonora/` the decisions are appended to.
pub const FILE: &str = "decisions.jsonl";

/// Where the decisions live: the platform's state folder, or its cache folder when it has no
/// state folder.
pub fn path() -> Option<PathBuf> {
    let root = dirs::state_dir().or_else(dirs::cache_dir)?;
    Some(root.join("sonora").join(FILE))
}

/// One thing the listener decided about the track that was playing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    /// Unix seconds.
    pub at: i64,
    #[serde(flatten)]
    pub action: Action,
    #[serde(flatten)]
    pub track: TrackRef,
}

/// What was decided.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case")]
pub enum Action {
    /// A rating from 1 to 5, or 0 for no rating.
    Rate { stars: u8 },
    /// Whether the track was saved to the library.
    Favorite { saved: bool },
    /// The track is to be left out of the library, not played again.
    Discard,
}

/// Enough of a track to find it again and to show a person what was decided.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackRef {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub album: String,
    /// The provider's slug, `local` for a file, `None` while it is unknown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

/// Appends `decision` to the file, making its folder when it is not there yet.
pub fn append(decision: &Decision) -> io::Result<()> {
    let Some(path) = path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(decision).map_err(io::Error::other)?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")
}

/// Reads the newest `limit` decisions, oldest first. Zero reads every one. A line that does not
/// parse, as a write cut short, is skipped rather than failing the whole read. A file that is
/// not there yet is no decisions.
pub fn read(limit: usize) -> io::Result<Vec<Decision>> {
    let Some(path) = path() else {
        return Ok(Vec::new());
    };
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut decisions: Vec<Decision> = BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str(&line).ok())
        .collect();
    if limit > 0 && decisions.len() > limit {
        decisions.drain(..decisions.len() - limit);
    }
    Ok(decisions)
}

#[cfg(test)]
mod tests {
    use super::{Action, Decision, TrackRef};

    #[test]
    fn a_decision_reads_flat() {
        let decision = Decision {
            at: 1_764_000_000,
            action: Action::Rate { stars: 4 },
            track: TrackRef {
                id: "42".into(),
                title: "梧桐灯".into(),
                artist: "许嵩".into(),
                album: "不如吃茶去".into(),
                provider: Some("subsonic".into()),
            },
        };
        let line = serde_json::to_string(&decision).unwrap();
        assert_eq!(
            line,
            r#"{"at":1764000000,"action":"rate","stars":4,"id":"42","title":"梧桐灯","artist":"许嵩","album":"不如吃茶去","provider":"subsonic"}"#
        );
        let read: Decision = serde_json::from_str(&line).unwrap();
        assert_eq!(read, decision);
    }
}
