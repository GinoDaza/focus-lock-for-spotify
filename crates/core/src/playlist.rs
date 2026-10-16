use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

const ID_LEN: usize = 22;
const URI_PREFIX: &str = "spotify:playlist:";
const WEB_HOST: &str = "open.spotify.com/";

/// A Spotify playlist ID: 22 base62 characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PlaylistId(String);

impl PlaylistId {
    /// Parses a bare ID, a `spotify:playlist:<id>` URI or an
    /// `https://open.spotify.com/playlist/<id>?si=...` share link.
    ///
    /// # Errors
    /// [`Error::InvalidPlaylist`] when no playlist ID can be extracted.
    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        let candidate = if let Some(rest) = trimmed.strip_prefix(URI_PREFIX) {
            rest
        } else if let Some((_, after_host)) = trimmed.split_once(WEB_HOST) {
            id_from_web_path(after_host)
        } else {
            trimmed
        };

        if candidate.len() == ID_LEN && candidate.bytes().all(|b| b.is_ascii_alphanumeric()) {
            Ok(Self(candidate.to_owned()))
        } else {
            Err(Error::InvalidPlaylist(input.to_owned()))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The `spotify:playlist:<id>` form used by the Spotify client.
    #[must_use]
    pub fn uri(&self) -> String {
        format!("{URI_PREFIX}{}", self.0)
    }
}

/// Finds the segment after `playlist` in a web path. Localized links look
/// like `intl-es/playlist/<id>`.
fn id_from_web_path(path: &str) -> &str {
    let path = path.split(['?', '#']).next().unwrap_or_default();
    let mut segments = path.split('/').filter(|segment| !segment.is_empty());
    while let Some(segment) = segments.next() {
        if segment == "playlist" {
            return segments.next().unwrap_or_default();
        }
    }
    ""
}

impl fmt::Display for PlaylistId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for PlaylistId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

impl TryFrom<String> for PlaylistId {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        Self::parse(&value)
    }
}

impl From<PlaylistId> for String {
    fn from(id: PlaylistId) -> Self {
        id.0
    }
}

/// The playlists allowed during a focus session. Never empty, no duplicates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<PlaylistId>", into = "Vec<PlaylistId>")]
pub struct Allowlist(Vec<PlaylistId>);

impl Allowlist {
    /// # Errors
    /// [`Error::EmptyAllowlist`] when `ids` is empty.
    pub fn new(ids: impl IntoIterator<Item = PlaylistId>) -> Result<Self> {
        let mut unique: Vec<PlaylistId> = Vec::new();
        for id in ids {
            if !unique.contains(&id) {
                unique.push(id);
            }
        }
        if unique.is_empty() {
            return Err(Error::EmptyAllowlist);
        }
        Ok(Self(unique))
    }

    #[must_use]
    pub fn contains(&self, id: &PlaylistId) -> bool {
        self.0.contains(id)
    }

    /// Whether a playback context URI from the client is allowed. Only
    /// `spotify:playlist:` URIs can match.
    #[must_use]
    pub fn allows_uri(&self, uri: &str) -> bool {
        uri.starts_with(URI_PREFIX) && PlaylistId::parse(uri).is_ok_and(|id| self.contains(&id))
    }

    pub fn iter(&self) -> impl Iterator<Item = &PlaylistId> {
        self.0.iter()
    }
}

impl TryFrom<Vec<PlaylistId>> for Allowlist {
    type Error = Error;

    fn try_from(ids: Vec<PlaylistId>) -> Result<Self> {
        Self::new(ids)
    }
}

impl From<Allowlist> for Vec<PlaylistId> {
    fn from(list: Allowlist) -> Self {
        list.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "37i9dQZF1DX8NTLI2TtZa6";
    const OTHER: &str = "0vvXsWCC9xrXsKd4FyS8kM";

    fn id(s: &str) -> PlaylistId {
        PlaylistId::parse(s).unwrap()
    }

    #[test]
    fn parses_every_supported_form() {
        for input in [
            ID.to_owned(),
            format!("spotify:playlist:{ID}"),
            format!("https://open.spotify.com/playlist/{ID}"),
            format!("https://open.spotify.com/playlist/{ID}?si=abc123"),
            format!("https://open.spotify.com/intl-es/playlist/{ID}?si=abc"),
            format!("  open.spotify.com/playlist/{ID}#x  "),
        ] {
            assert_eq!(id(&input).as_str(), ID, "input: {input}");
        }
    }

    #[test]
    fn rejects_non_playlists() {
        for input in [
            "",
            "hello",
            &format!("https://open.spotify.com/track/{ID}"),
            &format!("spotify:album:{ID}"),
            &format!("{ID}x"),
            "37i9dQZF1DX8NTLI2TtZa!",
        ] {
            assert!(PlaylistId::parse(input).is_err(), "input: {input}");
        }
    }

    #[test]
    fn uri_round_trips() {
        let playlist = id(ID);
        assert_eq!(playlist.uri(), format!("spotify:playlist:{ID}"));
        assert_eq!(id(&playlist.uri()), playlist);
    }

    #[test]
    fn allowlist_rejects_empty_and_dedupes() {
        assert_eq!(Allowlist::new([]), Err(Error::EmptyAllowlist));
        let list = Allowlist::new([id(ID), id(OTHER), id(ID)]).unwrap();
        assert_eq!(list.iter().count(), 2);
    }

    #[test]
    fn allowlist_matches_only_playlist_uris() {
        let list = Allowlist::new([id(ID)]).unwrap();
        assert!(list.allows_uri(&format!("spotify:playlist:{ID}")));
        assert!(!list.allows_uri(&format!("spotify:playlist:{OTHER}")));
        assert!(!list.allows_uri(&format!("spotify:album:{ID}")));
        assert!(!list.allows_uri(ID));
    }

    #[test]
    fn serde_validates_on_deserialize() {
        let json = serde_json::to_string(&Allowlist::new([id(ID)]).unwrap()).unwrap();
        assert_eq!(json, format!("[\"{ID}\"]"));
        assert!(serde_json::from_str::<Allowlist>("[]").is_err());
        assert!(serde_json::from_str::<PlaylistId>("\"nope\"").is_err());
    }
}
