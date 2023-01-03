use rand::distributions::Alphanumeric;
use rand::Rng;
use sha2::Digest;
use sha2::Sha256;

use crate::info;

const URL_SAFE_ENGINE: base64::engine::fast_portable::FastPortable =
    base64::engine::fast_portable::FastPortable::from(
        &base64::alphabet::URL_SAFE,
        base64::engine::fast_portable::NO_PAD,
    );

pub(crate) fn random_string(length: u8) -> String {
    let mut rng = rand::thread_rng();
    let mut s = String::new();
    for _ in 0..length {
        s.push(rng.sample(Alphanumeric) as char);
    }
    s
}

pub(crate) fn sha_256ify(s: &String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s);
    let res = hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    res
}

pub(crate) fn sha_256ify_raw(s: &String) -> Box<[u8]> {
    let mut hasher = Sha256::new();
    hasher.update(s);
    let res = hasher.finalize().iter().copied().collect();
    res
}

pub(crate) fn base64ify(s: String) -> String {
    base64::encode_engine(s, &URL_SAFE_ENGINE)
}

pub(crate) fn base64ify_raw(s: Box<[u8]>) -> String {
    base64::encode_engine(s, &URL_SAFE_ENGINE)
}

pub(crate) fn gen_code_challenge(s: &String) -> String {
    let result = base64ify_raw(sha_256ify_raw(s));
    info!("code_challenge: {}", result);
    result
}

pub(crate) fn rfc3339_to_epoch_time(s: &str) -> u64 {
    let s = s.replace('Z', "");
    let s = s.split('T').collect::<Vec<&str>>();
    let date = s[0]
        .split('-')
        .collect::<Vec<&str>>()
        .iter()
        .map(|s| s.parse::<u64>().unwrap())
        .collect::<Vec<u64>>();
    let time = s[1]
        .split(':')
        .collect::<Vec<&str>>()
        .iter()
        .map(|s| s.parse::<u64>().unwrap())
        .collect::<Vec<u64>>();
    time[0] * 3600
        + time[1] * 60
        + time[2]
        + date[2] * 86400
        + date[1] * 2592000
        + date[0] * 31104000_u64
}

pub(crate) fn epoch_time_to_rfc3339(t: u64) -> String {
    let mut t = t;
    let mut s = String::new();
    let years = t / 31104000;
    t -= years * 31104000;
    let months = t / 2592000;
    t -= months * 2592000;
    let days = t / 86400;
    t -= days * 86400;
    let hours = t / 3600;
    t -= hours * 3600;
    let minutes = t / 60;
    t -= minutes * 60;
    let seconds = t;
    s.push_str(&format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        years, months, days, hours, minutes, seconds
    ));
    s
}

pub(crate) fn strip_html_tags(str: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in str.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }
    result
}

pub(crate) fn format_duration(secs: u32) -> String {
    format!(
        "{}:{}",
        secs / 60,
        if secs % 60 < 10 {
            format!("0{}", secs % 60)
        } else {
            format!("{}", secs % 60)
        }
    )
}

pub struct Pair<A, B> {
    pub a: A,
    pub b: B,
}
