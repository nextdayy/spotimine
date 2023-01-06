use std::fmt::{Debug, Formatter};
use std::io;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::api::do_api_json;
use crate::utils::{base64ify, gen_code_challenge, random_string};
use crate::{info, SPOTIFY_CLIENT_ID};

#[derive(Serialize, Deserialize, Clone)]
pub struct Account {
    access_token: String,
    #[serde(alias = "expires_in")]
    expires_at: u64,
    refresh_token: String,
    #[serde(default = "id_default")]
    id: Option<String>,
    pub scope: String,
}

fn id_default() -> Option<String> {
    None
}

impl Account {
    fn is_valid(&self) -> bool {
        !self.access_token.is_empty() && !self.refresh_token.is_empty()
    }
    fn needs_refresh(&self) -> bool {
        !self.refresh_token.is_empty()
            && SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                > self.expires_at
    }

    pub(crate) fn get_token(&mut self) -> Result<&str, String> {
        if self.needs_refresh() {
            self.refresh()?;
            Ok(self.access_token.as_str())
        } else {
            Ok(self.access_token.as_str())
        }
    }

    pub(crate) fn get_id(&mut self) -> Result<&str, String> {
        if self.id.is_none() {
            self.id = Some(String::from(
                do_api_json("GET", "me", self, "")?["id"]
                    .as_str()
                    .ok_or("Failed to get user id")?,
            ));
            Ok(self.id.as_ref().unwrap())
        } else {
            Ok(self.id.as_ref().unwrap())
        }
    }

    pub(crate) fn new() -> Result<Account, String> {
        get_access()
    }

    pub(crate) fn refresh(&mut self) -> Result<&mut Account, String> {
        info!("Refreshing token");
        let result = ureq::post("https://accounts.spotify.com/api/token")
            .send_form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", self.refresh_token.as_str()),
                ("client_id", SPOTIFY_CLIENT_ID),
            ])
            .map_err(|e| {
                format!(
                    "failed to send token refresh request: {}. Try re-adding this account",
                    e.into_response()
                        .unwrap_or_else(|| "Tried to unwrap a completely broken response"
                            .parse()
                            .unwrap())
                        .into_string()
                        .unwrap_or_else(|_| "Tried to unwrap a completely broken response"
                            .parse()
                            .unwrap())
                )
            })?
            .into_string()
            .map_err(|e| format!("failed to get token refresh response: {}", e))?;
        let result: Account = serde_json::from_str(result.as_str())
            .map_err(|e| format!("failed to parse token response: {}", e))?;
        info!("Refreshed access token");
        self.access_token = result.access_token;
        self.expires_at = result.expires_at
            + SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
        self.refresh_token = result.refresh_token;
        Ok(self)
    }

    pub(crate) fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| format!("failed to serialize account: {}", e))
    }
}

fn get_access() -> Result<Account, String> {
    info!("Starting auth callback server");
    let listener = TcpListener::bind("127.0.0.1:8888").map_err(|e| e.to_string())?;
    let challenge = base64ify(random_string(64));
    let scope = "user-read-private user-read-email user-read-playback-state user-modify-playback-state user-read-currently-playing user-read-recently-played user-library-read user-library-modify user-top-read playlist-read-private playlist-read-collaborative playlist-modify-public playlist-modify-private";
    let mut request = format!("client_id={}&response_type=code&state={}&redirect_uri=http://localhost:8888/callback.html&code_challenge_method=S256&code_challenge={}&scope={}",
	    SPOTIFY_CLIENT_ID, random_string(16), 
	    gen_code_challenge(&challenge), scope);
    request = request
        .replace('/', "%2F")
        .replace(':', "%3A")
        .replace(' ', "+");
    let req = format!("https://accounts.spotify.com/authorize?{}", request);
    open::that(req).map_err(|_| "failed to open browser")?;
    get_token(callback(listener.accept())?, challenge)
}

fn callback(result: io::Result<(TcpStream, SocketAddr)>) -> Result<String, String> {
    info!("Got a callback request");
    return match result {
        Ok(mut stream) => {
            let mut s = String::new();
            BufReader::new(&mut stream.0)
                .read_line(&mut s)
                .map_err(|e| e.to_string())?;
            let data = "<!DOCTYPE html><html><head><title>Success</title></head><body><h1>Success</h1><p>You can now close this window.</p></body></html>";
            let _ = stream.0.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                    data.len(),
                    data
                )
                .as_bytes(),
            );
            info!("successfully got callback, shutting down server and getting token");
            let _ = stream.0.shutdown(std::net::Shutdown::Both);
            Ok(s)
        }
        Err(e) => Err(format!("Failed to establish connection {}", e)),
    };
}

fn get_token(result: String, challenge: String) -> Result<Account, String> {
    let code = result.split("code=").collect::<Vec<&str>>()[1]
        .split('&')
        .collect::<Vec<&str>>()[0];
    let result = ureq::post("https://accounts.spotify.com/api/token")
        .send_form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", "http://localhost:8888/callback.html"),
            ("client_id", SPOTIFY_CLIENT_ID),
            ("code_verifier", challenge.as_str()),
        ])
        .map_err(|e| format!("failed to send token request: {}", e))?
        .into_string()
        .map_err(|e| format!("failed to get token response: {}", e))?;
    info!("Got token response");
    let mut res: Account = serde_json::from_str(result.as_str()).map_err(|e| e.to_string())?;
    res.expires_at += SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    info!(
        "Account creation successful, requires refresh at {}",
        res.expires_at
    );
    Ok(res)
}

impl Debug for Account {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Account")
            .field("access_token", &self.access_token)
            .field("expires_at", &self.expires_at)
            .field("refresh_token", &self.refresh_token)
            .field("scope", &self.scope)
            .finish()
    }
}
