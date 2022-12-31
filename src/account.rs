use std::fmt::{Debug, Formatter};
use std::io;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::SystemTime;

use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::utils::{base64ify, gen_code_challenge, random_string};
use crate::{info, SPOTIFY_CLIENT_ID};

#[derive(Serialize, Deserialize)]
pub struct Account {
    pub access_token: String,
    #[serde(alias = "expires_in")]
    expires_at: Time,
    refresh_token: String,
    pub scope: String,
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
                > self.expires_at.val
    }

    pub(crate) fn get_token(&mut self) -> &str {
        if self.needs_refresh() {
            self.refresh().expect("Failed to refresh access token")
        } else {
            self.access_token.as_str()
        }
    }

    pub(crate) fn new() -> Account {
        let res = get_access().unwrap();
        res
    }

    fn refresh(&mut self) -> Result<&str, String> {
        let result = ureq::
            post("https://accounts.spotify.com/api/token")
            .send_form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", self.refresh_token.as_str()),
                ("client_id", SPOTIFY_CLIENT_ID),
            ])
            .map_err(|e| format!("failed to send token request: {}", e))?
            .into_string()
            .map_err(|e| format!("failed to get token response: {}", e))?;
        let result: Account = serde_json::from_str(result.as_str()).unwrap();
        self.access_token = result.access_token;
        self.expires_at = result.expires_at;
        self.refresh_token = result.refresh_token;
        return Ok(self.access_token.as_str());
    }

    pub(crate) fn to_json(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize account")
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
    request = request.replace("/", "%2F");
    request = request.replace(":", "%3A");
    request = request.replace(" ", "+");
    let req = format!("https://accounts.spotify.com/authorize?{}", request);
    //println!("{}", req);
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
    return serde_json::from_str(result.as_str()).map_err(|e| e.to_string());
}

#[derive(Debug)]
struct Time {
    val: u64,
}

impl Serialize for Time {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.val > 100000 {
            serializer.serialize_u64(self.val)
        } else {
            serializer.serialize_u64(
                self.val
                    + SystemTime::duration_since(&SystemTime::now(), SystemTime::UNIX_EPOCH)
                        .expect("failed to parse expires_in")
                        .as_secs(),
            )
        }
    }
}

struct TimeVisitor;

impl Visitor<'_> for TimeVisitor {
    type Value = Time;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("a u64")
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Time { val: v })
    }
}

impl<'de> Deserialize<'de> for Time {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_u64(TimeVisitor)
    }
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

/*pub(crate) fn from_json(json: String) -> Result<Account, String> {
    Ok(Account {
        // goofy ahh json parser
        access_token: json.split("access_token").collect::<Vec<&str>>()[1]
            .split(":")
            .collect::<Vec<&str>>()[1]
            .split(",")
            .collect::<Vec<&str>>()[0]
            .replace("\"", ""),
        expires_at: json.split("expires_in").collect::<Vec<&str>>()[1]
            .split(":")
            .collect::<Vec<&str>>()[1]
            .split(",")
            .collect::<Vec<&str>>()[0]
            .parse::<u64>()
            .expect("failed to parse expires_in")
            + SystemTime::duration_since(&SystemTime::now(), SystemTime::UNIX_EPOCH)
                .expect("failed to parse expires_in")
                .as_secs(),
        refresh_token: json.split("refresh_token").collect::<Vec<&str>>()[1]
            .split(":")
            .collect::<Vec<&str>>()[1]
            .split(",")
            .collect::<Vec<&str>>()[0]
            .replace("\"", ""),
        scope: json.split("scope").collect::<Vec<&str>>()[1]
            .split(":")
            .collect::<Vec<&str>>()[1]
            .split(",")
            .collect::<Vec<&str>>()[0]
            .replace("\"", ""),
        aliases: Vec::new(),
    })
}*/
