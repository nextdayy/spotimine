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
