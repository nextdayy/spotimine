use std::fs::File;
use std::io;
use std::io::Write;

use colored::Colorize;
use serde_json::Value;

use crate::account::Account;
use crate::api::{do_api, spotify_api_search};
use crate::config::{load, Config};
use crate::data::{Album, Artist, ContentTypes, Playlist, Track};

mod account;
mod api;
mod config;
mod data;
mod utils;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const SPOTIFY_CLIENT_ID: &str = "d75a5cecbe5c4b71869c602e802ba265";

struct Spotimine {
    file: File,
    config: Config,
}

impl Spotimine {
    fn new(config_file_path: String) -> Result<Spotimine, String> {
        let config = Config::init(config_file_path.as_str())?;
        Ok(Spotimine {
            file: config.a,
            config: config.b,
        })
    }
}

fn main() {
    println!(
        "{} v{} by {}; running on {}",
        "spotimine".green().bold(),
        VERSION.bold(),
        AUTHORS.bold(),
        std::env::consts::OS.bold()
    );
    println!(
        "For instructions and how to use, please visit {}.",
        "https://github.com/nxtdaydelivery/spotimine"
            .blue()
            .underline()
    );
    let mut this = load().expect("Failed to initialize");
    loop {
        print!("{}", "spotimine> ".green());
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        dispatch(input.as_str().trim(), &mut this);
    }
}

fn dispatch(command: &str, this: &mut Spotimine) {
    let args = command.split(' ').collect::<Vec<&str>>();
    match args[0] {
        "" => {}
        "exit" => {
            println!("{}", "Exiting...".red());
            exit(0, this);
        }
        "adduser" => {
            if args.len() == 1 {
                let mut account = Account::new();
                match do_api("GET", "me", &mut account, None) {
                    Ok(res) => {
                        this.config.add_account(
                            &mut this.file,
                            res.into_json::<Value>().unwrap()["display_name"]
                                .as_str()
                                .unwrap(),
                            account,
                        );
                    }
                    Err(_) => {
                        error!("Failed to get account info from Spotify API.");
                    }
                }
            } else if check_args_len(&args, 1) {
                this.config
                    .add_account(&mut this.file, args[1], Account::new());
            }
        }
        "rmuser" => {
            if check_args_len(&args, 1) {
                this.config.remove_account(&mut this.file, args[1]);
            }
        }
        "search" => {
            if check_args_len(&args, 2) {
                let account = this.config.get_an_account();
                if account.is_none() {
                    error!("No accounts found. At least one is required to use the API. Try using 'adduser'");
                    return;
                }
                match ContentTypes::from_str(args[1]) {
                    Some(typ) => {
                        match typ {
                            ContentTypes::Tracks => spotify_api_search::<Track>(args[2], &typ, account.unwrap()).unwrap().iter().for_each(|x| println!("{}", x)),
                            ContentTypes::Albums => spotify_api_search::<Album>(args[2], &typ, account.unwrap()).unwrap().iter().for_each(|x| println!("{:?}", x)),
                            ContentTypes::Artists => spotify_api_search::<Artist>(args[2], &typ, account.unwrap()).unwrap().iter().for_each(|x| println!("{:?}", x)),
                            ContentTypes::Playlists => spotify_api_search::<Playlist>(args[2], &typ, account.unwrap()).unwrap().iter().for_each(|x| println!("{:?}", x)),
                        }
                    }
                    None => error!("Invalid content type. Valid types are: 'track', 'album', 'artist', 'playlist'"),
                }
            }
        }
        "config" => {
            println!("config file is {:?}", this.file);
        }
        "users" => {
            println!("Found {} users:", this.config.accounts.len());
            for (key, acc) in this.config.accounts.iter() {
                let mut id = acc.access_token.clone();
                id.truncate(20);
                id.push_str("...");
                println!("{}: {}", key, id);
            }
        }
        "delusers" => {
            if user_yn(
                "Are you sure you want to delete ALL accounts? This cannot be undone!",
                false,
            ) {
                this.config.accounts.clear();
                this.config
                    .save_to(&mut this.file)
                    .expect("Failed to save config");
                info!("deleted all users.");
            }
        }
        "testyn" => {
            println!("{}", user_yn("Test? ", false));
        }
        _ => error!("Unknown command: '{}'", command),
    }
}

fn user_yn(prompt: &str, default: bool) -> bool {
    println!();
    let mut input = String::new();
    print!("{} [{}]: ", prompt, if default { "Y/n" } else { "y/N" });
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim();
    if input.is_empty() {
        return default;
    }
    return match input.to_lowercase().as_str() {
        "y" => true,
        "n" => false,
        _ => default,
    };
}

fn check_args_len(args: &Vec<&str>, len: usize) -> bool {
    if args.len() < len + 1 {
        error!(
            "Not enough arguments. Expected {}, got {}.",
            len,
            args.len() - 1
        );
        return false;
    }
    true
}

fn exit(code: i8, this: &mut Spotimine) {
    this.config
        .save_to(&mut this.file)
        .expect("Failed to save config while exiting, users may be corrupt!");
    std::process::exit(code as i32);
}

fn info(message: String) {
    println!("{} {}", "[INFO]".bold(), message);
}

fn error(message: String) {
    println!("{} {}", "Error:".red().bold(), message.red().italic());
}

fn fatal(message: String) {
    println!("{} {}", "FATAL:".red().bold(), message.red().italic());
}

fn warn(message: String) {
    println!(
        "{} {}",
        "Warning:".yellow().bold(),
        message.yellow().italic()
    );
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => (info(format!($($arg)*)));
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => (error(format!($($arg)*)));
}

#[macro_export]
macro_rules! fatal {
    ($($arg:tt)*) => (fatal(format!($($arg)*)));
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => (warn(format!($($arg)*)));
}
