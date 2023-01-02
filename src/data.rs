use crate::account::Account;
use crate::api::RequestExt;
use colored::Colorize;
use serde_json::Value;
use std::fmt::{Display, Formatter};
use std::time::Duration;
use crate::utils::format_duration;

#[derive(Debug, Clone)]
pub struct Playlist {
    pub name: String,
    pub description: String,
    pub visibility: Visibility,
    pub followers: u32,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone)]
pub struct Track {
    pub name: String,
    pub artists: Vec<Artist>,
    pub duration: u32,
    pub explicit: bool,
    pub uri: SpotifyURI,
}

#[derive(Debug, Clone)]
pub struct User {
    pub followers: u32,
    pub uri: SpotifyURI,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Album {
    pub name: String,
    pub artists: Vec<Artist>,
    pub tracks: Vec<Track>,
    pub uri: SpotifyURI,
}

#[derive(Debug, Clone)]
pub struct Artist {
    pub name: String,
    pub uri: SpotifyURI,
}

#[derive(Debug, Clone)]
pub struct SpotifyURI {
    pub uri: String,
}

impl SpotifyURI {
    pub fn from_str(uri: String) -> SpotifyURI {
        SpotifyURI { uri }
    }
}

#[derive(Debug, Clone)]
pub enum Visibility {
    Public,
    Private,
    Collaborative,
}

impl Visibility {
    pub fn from_api(collaborative: bool, public: bool) -> Visibility {
        if collaborative {
            Visibility::Collaborative
        } else if public {
            Visibility::Public
        } else {
            Visibility::Private
        }
    }
}

pub enum ContentTypes {
    Tracks,
    Artists,
    Albums,
    Playlists,
}

impl ContentTypes {
    pub fn from_str(s: &str) -> Option<ContentTypes> {
        match s.to_lowercase().as_str() {
            "track" | "song" | "tracks" | "songs" => Some(ContentTypes::Tracks),
            "artist" | "singer" | "artists" | "singers" => Some(ContentTypes::Artists),
            "album" | "albums" => Some(ContentTypes::Albums),
            "playlist" | "list" | "playlists" => Some(ContentTypes::Playlists),
            _ => None,
        }
    }
    pub fn to_str_plural(&self) -> &str {
        match self {
            ContentTypes::Tracks => "tracks",
            ContentTypes::Artists => "artists",
            ContentTypes::Albums => "albums",
            ContentTypes::Playlists => "playlists",
        }
    }
    pub fn to_str(&self) -> &str {
        match self {
            ContentTypes::Tracks => "track",
            ContentTypes::Artists => "artist",
            ContentTypes::Albums => "album",
            ContentTypes::Playlists => "playlist",
        }
    }
}

pub trait Content: Sized {
    fn from_json(json: &Value) -> Self;
    fn from_id(id: &str, user: &mut Account, typ: &ContentTypes) -> Self {
        let result =
            ureq::get(format!("https://api.spotify.com/v1/{}/{}", typ.to_str(), id).as_str())
                .add_auth(user)
                .call();
        let json = result.unwrap().into_json::<Value>().unwrap();
        Self::from_json(&json)
    }
    fn from_json_array(json: &Value) -> Vec<Self> {
        let mut vec = Vec::new();
        for item in json.as_array().unwrap() {
            vec.push(Self::from_json(item));
        }
        vec
    }
}

impl Display for Track {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "{} by {} ({})",
            self.name.blue().bold(),
            self.artists[0].name.blue().bold(),
            format_duration(self.duration)
        ))
    }
}

impl Content for Track {
    fn from_json(json: &Value) -> Self {
        let mut artists = Vec::new();
        for artist in json["artists"].as_array().unwrap() {
            artists.push(Artist::from_json(artist));
        }
        Track {
            name: json["name"].as_str().unwrap().to_string(),
            artists,
            duration: (json["duration_ms"].as_u64().unwrap() / 1000) as u32,
            explicit: json["explicit"].as_bool().unwrap(),
            uri: SpotifyURI::from_str(json["uri"].as_str().unwrap().to_string()),
        }
    }
}

impl Content for Artist {
    fn from_json(json: &Value) -> Self {
        Artist {
            name: json["name"].as_str().unwrap().to_string(),
            uri: SpotifyURI::from_str(json["uri"].as_str().unwrap().to_string()),
        }
    }
}

impl Content for Album {
    fn from_json(json: &Value) -> Self {
        Album {
            name: json["name"].as_str().unwrap().to_string(),
            artists: Artist::from_json_array(&json["artists"]),
            tracks: Track::from_json_array(&json["tracks"]["items"]),
            uri: SpotifyURI::from_str(json["uri"].as_str().unwrap().to_string()),
        }
    }
}

impl Content for Playlist {
    fn from_json(json: &Value) -> Self {
        Playlist {
            name: json["name"].as_str().unwrap().to_string(),
            description: json["description"].as_str().unwrap().to_string(),
            visibility: Visibility::from_api(
                json["collaborative"].as_bool().unwrap_or(false),
                json["public"].as_bool().unwrap_or(false),
            ),
            followers: json["followers"]["total"].as_u64().unwrap() as u32,
            tracks: Track::from_json_array(&json["tracks"]["items"]),
        }
    }
}
