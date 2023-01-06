use serde_json::Value;
use ureq::{Error, Request, Response};

use crate::account::Account;
use crate::data::{Content, ContentType, Playlist, PlaylistTrack, SpotifyURI, Visibility};
use crate::{info, warn};

pub trait RequestExt {
    fn add_auth(self, account: &mut Account) -> Result<Request, String>;
}

impl RequestExt for Request {
    fn add_auth(self, account: &mut Account) -> Result<Request, String> {
        Ok(self.set(
            "Authorization",
            format!("Bearer {}", account.get_token()?).as_str(),
        ))
    }
}

pub fn do_api(
    method: &str,
    endpoint: &str,
    account: &mut Account,
    json: impl serde::Serialize,
) -> Result<Response, String> {
    let req = ureq::request(
        method,
        format!("https://api.spotify.com/v1/{}", endpoint).as_str(),
    )
    .add_auth(account);
    let response = match method {
        "GET" => req?.call(),
        "POST" => req?.send_json(&json),
        "PUT" => req?.send_json(&json),
        "DELETE" => req?.send_json(&json),
        _ => panic!("Invalid method"),
    };
    return match response {
        Ok(response) => Ok(response),
        Err(err) => match err {
            Error::Status(code, response) => {
                return match code {
                        401 => {
                            do_api(method, endpoint, account.refresh().expect("Failed to refresh access token"), json)
                        }
                        403 => {
                            Err(format!("User account {}'s OAuth is invalid. Please try re-adding this account, then try again. Response {}", account.get_id()?, response.into_string().expect("Failed to unwrap broken 403 response")))
                        }
                        423 | 429 => {
                            let retry_after = match response.header("Retry-After") {
                                Some(val) => val.parse::<u64>().unwrap(),
                                None => 5,
                            };
                            warn!("Spotify API rate limit exceeded, retrying in {} seconds", retry_after);
                            std::thread::sleep(std::time::Duration::from_secs(retry_after));
                            do_api(method, endpoint, account, json)
                        }
                        400..=499 => Err(format!("Client error: {} (code {})", &response.into_string().expect("Too many things went wrong during API request: failed to parse a 400 series error code response"), code)),
                        500..=599 => Err(format!("Server error: {} (code {})", &response.into_string().expect("Too many things went wrong during API request: failed to parse a 500 series error code response"), code)),
                        _ => Err(format!("Unknown error: {}", response.into_string().expect("Too many things went wrong during API request: response code out of range"))),
                    }
            }
            _ => Err(format!("Failed to send request: {}", err)),
        }
    };
}

pub fn do_api_json(
    method: &str,
    endpoint: &str,
    account: &mut Account,
    body: impl serde::Serialize,
) -> Result<Value, String> {
    let response = do_api(method, endpoint, account, body)?;
    let json = response
        .into_json()
        .map_err(|e| format!("failed to parse response: {}", e))?;
    Ok(json)
}

pub fn spotify_api_search<T: Content>(
    query: &str,
    t: &ContentType,
    account: &mut Account,
) -> Result<Vec<T>, String> {
    return match t {
        ContentType::Playlists => {
            let json = do_api_json(
                "GET",
                format!("search?q={}&type=playlist", query).as_str(),
                account,
                "",
            )?;
            let playlist_ids = json["playlists"]["items"]
                .as_array()
                .ok_or("Failed to parse playlists")?
                .iter()
                .map(|v| v["id"].as_str())
                .collect::<Vec<Option<&str>>>();
            let mut results: Vec<T> = Vec::new();
            for id in playlist_ids {
                if id.is_some() {
                    results.push(T::from_id(id.unwrap(), account)?);
                }
            }
            Ok(results)
        }
        _ => T::from_json_array(
            &do_api_json(
                "GET",
                format!("search?q={}&type={}", query, t.to_str()).as_str(),
                account,
                "",
            )?[t.to_str_plural()]["items"],
        ),
    };
}

pub fn get_playlists_for(acc: &mut Account) -> Result<Vec<Playlist>, String> {
    info!("Getting playlists. This may take a while, as we need to fetch all the tracks.");
    let mut playlists = Vec::new();
    let _ = do_api_json("GET", "me/playlists?limit=50", acc, "")?["items"]
        .as_array()
        .ok_or("Failed to get playlists")?
        .iter()
        .try_for_each(|p| -> Result<(), String> {
            playlists.push(Playlist::from_id(
                p["id"].as_str().ok_or("no ID field")?,
                acc,
            )?);
            Ok(())
        });
    Ok(playlists)
}

pub fn get_liked_songs(acc: &mut Account) -> Result<Playlist, String> {
    let mut tracks = Vec::new();
    let mut offset = 0;
    let json = do_api_json("GET", "me/tracks?limit=50", acc, "")?;
    let total = json["total"].as_u64().ok_or("Failed to get total")?;
    info!("Getting {} liked songs. This may take a while.", total);
    let mut t = PlaylistTrack::from_json_array(&json["items"])?;
    offset += t.len();
    tracks.append(&mut t);
    while offset < total as usize {
        let json = do_api_json(
            "GET",
            format!("me/tracks?limit=50&offset={}", offset).as_str(),
            acc,
            "",
        )?;
        let mut t = PlaylistTrack::from_json_array(&json["items"])?;
        offset += t.len();
        tracks.append(&mut t);
    }
    Ok(Playlist {
        name: "Liked Songs".to_string(),
        description: "your liked songs".to_string(),
        visibility: Visibility::Private,
        followers: 0,
        tracks,
        uri: SpotifyURI {
            uri: "".to_string(),
        },
    })
}
