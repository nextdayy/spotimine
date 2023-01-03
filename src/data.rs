use colored::Colorize;
use serde_json::Value;
use std::fmt::{Display, Formatter};
use crate::account::Account;
use crate::api::do_api_json;

use crate::utils::{format_duration, rfc3339_to_duration, strip_html_tags};

#[derive(Debug, Clone)]
pub struct Playlist {
    pub name: String,
    pub description: String,
    pub visibility: Visibility,
    pub followers: u32,
    pub tracks: Vec<PlaylistTrack>,
}

#[derive(Debug, Clone)]
pub struct PlaylistTrack {
    pub track: Track,
    pub added_at: u64,
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

trait Stringify {
    fn stringify(&self) -> String;
}
impl Stringify for Vec<Artist> {
    fn stringify(&self) -> String {
        self.iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<&str>>()
            .join(", ")
    }
}

pub enum ContentType {
    Tracks,
    Artists,
    Albums,
    Playlists,
}

impl ContentType {
    pub fn from_str(s: &str) -> Option<ContentType> {
        match s.to_lowercase().as_str() {
            "track" | "song" | "tracks" | "songs" => Some(ContentType::Tracks),
            "artist" | "singer" | "artists" | "singers" => Some(ContentType::Artists),
            "album" | "albums" => Some(ContentType::Albums),
            "playlist" | "list" | "playlists" => Some(ContentType::Playlists),
            _ => None,
        }
    }
    pub fn to_str_plural(&self) -> &str {
        match self {
            ContentType::Tracks => "tracks",
            ContentType::Artists => "artists",
            ContentType::Albums => "albums",
            ContentType::Playlists => "playlists",
        }
    }
    pub fn to_str(&self) -> &str {
        match self {
            ContentType::Tracks => "track",
            ContentType::Artists => "artist",
            ContentType::Albums => "album",
            ContentType::Playlists => "playlist",
        }
    }
}

pub trait Content: Sized {
    fn from_json(json: &Value) -> Result<Self, String>;
    fn from_json_array(json: &Value) -> Result<Vec<Self>, String> {
        let mut vec = Vec::new();
        for item in json.as_array().ok_or(format!("json was not an array: {}", json))? {
            vec.push(Self::from_json(item)?);
        }
        Ok(vec)
    }
    fn from_id(id: &str, user: &mut Account) -> Result<Self, String> {
        Self::from_json(&do_api_json("GET", format!("{}s/{}", Self::type_string(), id).as_str(), user, None)?)
    }
    fn type_string() -> String;
}

impl Display for Track {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "{} by {} ({})",
            self.name.blue().bold(),
            self.artists.stringify().blue(),
            format_duration(self.duration)
        ))
    }
}

impl Display for Artist {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}", self.name.blue().bold()))
    }
}

impl Display for Album {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "{} by {}",
            self.name.blue().bold(),
            self.artists.stringify().blue()
        ))
    }
}

impl Display for Playlist {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if !self.description.is_empty() {
            f.write_str(&format!(
                "{} - {} ({} followers)",
                self.name.blue().bold(),
                strip_html_tags(&self.description).blue(),
                self.followers
            ))
        } else {
            f.write_str(&format!(
                "{} ({} followers)",
                self.name.blue().bold(),
                self.followers
            ))
        }
    }
}

impl Content for Track {
    fn from_json(json: &Value) -> Result<Self, String> {
        let mut artists = Vec::new();
        for artist in json["artists"].as_array().ok_or(format!(
            "Expected array deserializing artists: data = {}",
            json
        ))? {
            artists.push(Artist::from_json(artist)?);
        }
        Ok(Track {
            name: json["name"].as_str().unwrap().to_string(),
            artists,
            duration: (json["duration_ms"].as_u64().unwrap() / 1000) as u32,
            explicit: json["explicit"].as_bool().unwrap(),
            uri: SpotifyURI::from_str(json["uri"].as_str().unwrap().to_string()),
        })
    }

    fn type_string() -> String {
        String::from("track")
    }
}

impl Content for PlaylistTrack {
    fn from_json(json: &Value) -> Result<Self, String> {
        Ok(PlaylistTrack {
            track: Track::from_json(&json["track"])?,
            added_at: rfc3339_to_duration(json["added_at"].as_str().ok_or("timestamp missing")?),
        })
    }
    fn type_string() -> String {
        String::from("track")
    }
}

impl Content for Artist {
    fn from_json(json: &Value) -> Result<Self, String> {
        Ok(Artist {
            name: json["name"]
                .as_str()
                .ok_or("missing name field?")?
                .to_string(),
            uri: SpotifyURI::from_str(json["uri"].as_str().unwrap().to_string()),
        })
    }
    fn type_string() -> String {
        String::from("artist")
    }
}

impl Content for Album {
    fn from_json(json: &Value) -> Result<Self, String> {
        Ok(Album {
            name: json["name"]
                .as_str()
                .ok_or("missing name field?")?
                .to_string(),
            artists: Artist::from_json_array(&json["artists"])?,
            tracks: Track::from_json_array(&json["tracks"]["items"])?,
            uri: SpotifyURI::from_str(
                json["uri"]
                    .as_str()
                    .ok_or("missing URI field?")?
                    .to_string(),
            ),
        })
    }
    fn type_string() -> String {
        String::from("album")
    }
}

impl Content for Playlist {
    fn from_json(json: &Value) -> Result<Self, String> {
        let tracks = &mut json["tracks"]["items"]
            .as_array()
            .ok_or(format!(
                "Expected array deserializing tracks: data = {}",
                json
            ))?
            .iter()
            .collect::<Vec<&Value>>();
        tracks.retain(|x| !x["track"].is_null());
        let tracks: Vec<PlaylistTrack> = tracks
            .iter()
            .map(|x| PlaylistTrack::from_json(x))
            .collect::<Result<Vec<PlaylistTrack>, String>>()?;
        Ok(Playlist {
            name: json["name"]
                .as_str()
                .ok_or("missing name field?")?
                .to_string(),
            description: json["description"]
                .as_str()
                .ok_or("missing description field?")?
                .to_string()
                .trim()
                .parse()
                .map_err(|_| "description failed to parse")?,
            visibility: Visibility::from_api(
                json["collaborative"].as_bool().unwrap_or(false),
                json["public"].as_bool().unwrap_or(false),
            ),
            followers: json["followers"]["total"].as_u64().unwrap_or(0) as u32,
            tracks,
        })
    }
    fn type_string() -> String {
        String::from("playlist")
    }
}
