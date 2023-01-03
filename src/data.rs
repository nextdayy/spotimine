use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::account::Account;
use crate::api::{do_api, do_api_json};
use crate::info;
use crate::utils::{format_duration, rfc3339_to_epoch_time, strip_html_tags};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub name: String,
    pub description: String,
    pub visibility: Visibility,
    pub followers: u32,
    pub tracks: Vec<PlaylistTrack>,
    pub uri: SpotifyURI,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistTrack {
    pub track: Track,
    pub added_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub name: String,
    pub artists: Vec<Artist>,
    pub duration: u32,
    pub explicit: bool,
    pub uri: SpotifyURI,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub followers: u32,
    pub uri: SpotifyURI,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub name: String,
    pub artists: Vec<Artist>,
    pub tracks: Vec<Track>,
    pub uri: SpotifyURI,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artist {
    pub name: String,
    pub uri: SpotifyURI,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyURI {
    pub uri: String,
}

impl SpotifyURI {
    pub fn from_str(uri: String) -> SpotifyURI {
        SpotifyURI { uri }
    }
    pub fn get_id(&self) -> String {
        self.uri.split(':').last().unwrap().to_string()
    }
    pub fn get_type(&self) -> ContentType {
        ContentType::from_str(self.uri.split(':').nth(1).unwrap()).expect("Invalid URI")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub fn is_public(&self) -> bool {
        match self {
            Visibility::Public => true,
            _ => false,
        }
    }
    pub fn is_collaborative(&self) -> bool {
        match self {
            Visibility::Collaborative => true,
            _ => false,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        for item in json
            .as_array()
            .ok_or(format!("json was not an array: {}", json))?
        {
            vec.push(Self::from_json(item)?);
        }
        Ok(vec)
    }
    fn from_id(id: &str, user: &mut Account) -> Result<Self, String> {
        Self::from_json(&do_api_json(
            "GET",
            format!("{}s/{}", Self::type_string(), id).as_str(),
            user,
            None,
        )?)
    }
    fn from_ids(ids: &[&str], user: &mut Account) -> Result<Vec<Self>, String> {
        let mut vec = Vec::new();
        let mut vec_ids: Vec<&str> = Vec::new();
        for id in ids {
            vec_ids.push(id);
            if vec_ids.len() == 50 {
                vec.append(&mut Self::from_json_array(&do_api_json(
                    "GET",
                    format!("{}s/?ids={}", Self::type_string(), vec_ids.join(",")).as_str(),
                    user,
                    None,
                )?)?);
                vec_ids.clear();
            }
        }
        Ok(vec)
    }
    fn type_string() -> String;
    fn get_uri(&self) -> &SpotifyURI;
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
    fn get_uri(&self) -> &SpotifyURI {
        &self.uri
    }
}

impl Content for PlaylistTrack {
    fn from_json(json: &Value) -> Result<Self, String> {
        Ok(PlaylistTrack {
            track: Track::from_json(&json["track"])?,
            added_at: rfc3339_to_epoch_time(json["added_at"].as_str().ok_or("timestamp missing")?),
        })
    }
    fn type_string() -> String {
        String::from("track")
    }
    fn get_uri(&self) -> &SpotifyURI {
        &self.track.uri
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
    fn get_uri(&self) -> &SpotifyURI {
        &self.uri
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
    fn get_uri(&self) -> &SpotifyURI {
        &self.uri
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
            uri: SpotifyURI::from_str(
                json["uri"]
                    .as_str()
                    .ok_or("missing URI field?")?
                    .to_string(),
            ),
            tracks,
        })
    }
    fn type_string() -> String {
        String::from("playlist")
    }
    fn get_uri(&self) -> &SpotifyURI {
        &self.uri
    }
}

impl Playlist {
    pub fn to_file(&self, path: &Path) -> Result<(), String> {
        let mut file = File::create(path).map_err(|e| e.to_string())?;
        file.write_all(
            serde_json::to_string_pretty(self)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&contents).map_err(|e| e.to_string())
    }
    pub fn get_ids(&self) -> Vec<String> {
        self.tracks
            .iter()
            .map(|x| x.track.uri.uri.clone())
            .collect()
    }

    fn copy(
        &self,
        owner: &mut Account,
        new_name: Option<&str>,
        new_user: Option<&mut Account>,
    ) -> Result<Playlist, String> {
        let mut new_playlist = Playlist {
            name: new_name.unwrap_or(&self.name).to_string(),
            description: (&self.description).to_string(),
            visibility: self.visibility.clone(),
            followers: self.followers,
            tracks: vec![],
            uri: SpotifyURI {
                uri: "".to_string(),
            },
        };
        let user = new_user.unwrap_or(owner);
        new_playlist.create_online(user)?;
        let tracks: Vec<Track> = self.tracks.iter().map(|x| x.track.clone()).collect();
        new_playlist.put_tracks(owner, &tracks)?;
        Ok(new_playlist)
    }

    pub fn put_tracks(&mut self, user: &mut Account, tracks: &Vec<Track>) -> Result<(), String> {
        self.tracks.push(PlaylistTrack {
            track: tracks[0].clone(),
            added_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });
        let mut s = String::new();
        let mut i: u8 = 0;
        let mut ii: u8 = 0;
        info!(
            "Adding tracks to playlist... ({}/{})",
            ii * 100,
            tracks.len()
        );
        for track in tracks {
            if i == 100 {
                do_api(
                    "POST",
                    &format!("playlists/{}/tracks?uris={}", self.uri.get_id(), s),
                    user,
                    None,
                )?;
                s = String::new();
                i = 0;
                ii += 1;
            }
            s.push_str(&track.uri.uri);
            s.push(',');
            i += 1;
        }
        Ok(())
    }

    fn create_online(&mut self, user: &mut Account) -> Result<(), String> {
        self.uri = SpotifyURI::from_str(
            do_api_json(
                "POST",
                format!("users/{}/playlists", user.get_id()?).as_str(),
                user,
                Some(&[
                    ("name", &self.name.as_str()),
                    ("description", &self.description.as_str()),
                    ("public", &self.visibility.is_public().to_string()),
                    (
                        "collaborative",
                        &self.visibility.is_collaborative().to_string(),
                    ),
                ]),
            )?["uri"]
                .as_str()
                .ok_or("missing URI field, when creating playlist, probably invalid response")?
                .to_string(),
        );
        Ok(())
    }
}
