use serde_json::Value;
use ureq::{Error, Request, Response};

use crate::account::Account;
use crate::data::{Content, ContentType};
use crate::warn;

pub trait RequestExt {
    fn add_auth(self, account: &mut Account) -> Self;
}

impl RequestExt for Request {
    fn add_auth(self, account: &mut Account) -> Request {
        self.set(
            "Authorization",
            format!("Bearer {}", account.get_token()).as_str(),
        )
    }
}

pub fn do_api(
    method: &str,
    endpoint: &str,
    account: &mut Account,
    body: Option<&[(&str, &str)]>,
) -> Result<Response, String> {
    let req = ureq::request(
        method,
        format!("https://api.spotify.com/v1/{}", endpoint).as_str(),
    )
    .add_auth(account);
    let response = match method {
        "GET" => req.call(),
        "POST" => req.send_form(body.unwrap()),
        "PUT" => req.send_form(body.unwrap()),
        "DELETE" => req.call(),
        _ => panic!("Invalid method"),
    };
    match response {
        Ok(response) => Ok(response),
        Err(err) => {
            match err {
                Error::Status(code, response) => {
                    match code {
                        401 => {
                            do_api(method, endpoint, account.refresh().expect("Failed to refresh access token"), body)
                        }
                        403 => {
                            Err("User account OAuth is invalid. Please try re-adding this account, then try again.".to_string())
                        }
                        423 => {
                            let retry_after = match response.header("Retry-After") {
                                Some(val) => val.parse::<u64>().unwrap(),
                                None => 5,
                            };
                            warn!("Spotify API rate limit exceeded, retrying in {} seconds", retry_after);
                            std::thread::sleep(std::time::Duration::from_secs(retry_after));
                            do_api(method, endpoint, account, body)
                        }
                        400..=499 => Err(format!("Client error: {} (code {})", &response.into_string().expect("Too many things went wrong during API request: failed to parse a 400 series error code response"), code)),
                        500..=599 => Err(format!("Server error: {} (code {})", &response.into_string().expect("Too many things went wrong during API request: failed to parse a 500 series error code response"), code)),
                        _ => Err(format!("Unknown error: {}", response.into_string().expect("Too many things went wrong during API request: response code out of range"))),
                    }
                }
                _ => Err(format!("Failed to send request: {}", err)),
            }
        }

    }
}

pub fn do_api_json(
    method: &str,
    endpoint: &str,
    account: &mut Account,
    body: Option<&[(&str, &str)]>,
) -> Result<Value, String> {
    let response = do_api(method, endpoint, account, body)?;
    let json = response
        .into_json()
        .map_err(|e| format!("failed to parse response: {}", e))?;
    //println!("{}", json);
    Ok(json)
}

pub fn spotify_api_search<T: Content>(
    query: &str,
    t: &ContentType,
    account: &mut Account,
) -> Result<Vec<T>, String> {
    Ok(T::from_json_array(
        &do_api_json(
            "GET",
            format!("search?q={}&type={}", query, t.to_str()).as_str(),
            account,
            None,
        )?[t.to_str_plural()]["items"],
    ))
}
