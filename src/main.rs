use std::fmt::Display;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;

use colored::Colorize;
use serde_json::Value;

use crate::account::Account;
use crate::api::{do_api, do_api_json, spotify_api_search};
use crate::config::{load, Config};
use crate::data::{Album, Artist, Content, ContentType, Playlist, Track};

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
        let config = Config::init(Path::new(config_file_path.as_str()))?;
        Ok(Spotimine {
            file: config.a,
            config: config.b,
        })
    }
}

impl Drop for Spotimine {
    fn drop(&mut self) {
        info!("Saving config...");
        self.config
            .save_to(&mut self.file)
            .expect("Failed to save config");
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
        match dispatch(input.as_str().trim(), &mut this) {
            Ok(()) => (),
            Err(e) => {
                error!("{}", e.red())
            }
        }
    }
}

fn dispatch(command: &str, this: &mut Spotimine) -> Result<(), String> {
    let args = command.split(' ').collect::<Vec<&str>>();
    match args[0] {
        "" => Ok(()),
        "exit" => {
            println!("{}", "Exiting...".red());
            exit(0, this);
            Ok(())
        }
        "adduser" => {
            if args.len() == 1 {
                let mut account = Account::new()?;
                match do_api("GET", "me", &mut account, None) {
                    Ok(res) => this.config.add_account(
                        &mut this.file,
                        res.into_json::<Value>().unwrap()["display_name"]
                            .as_str()
                            .ok_or("Failed to get display name")?,
                        account,
                    ),
                    Err(_) => Err("Failed to get account info from Spotify API."
                        .parse()
                        .unwrap()),
                }
            } else {
                check_args_len(&args, 1)?;
                this.config
                    .add_account(&mut this.file, args[1], Account::new()?)
            }
        }
        "rmuser" => {
            check_args_len(&args, 1)?;
            this.config.remove_account(&mut this.file, args[1])
        }
        "copy" => {
            check_args_len(&args, 2)?;
            //user_choose()
            Ok(())
        }
        "playlists" => {
            check_args_len(&args, 2)?;
            let acc = this.config.get_account(args[1]);
            if acc.is_none() {
                return Err(format!(
                    "Account not found: {}. Try adding one with 'adduser'",
                    args[1]
                ));
            }
            info!("Getting playlists for {}. This may take a while, as we need to fetch all the tracks.", args[1]);
            let acc = acc.ok_or("Failed to get account")?;
            do_api_json(
                "GET",
                format!("users/{}/playlists", acc.get_id()?).as_str(),
                acc,
                None,
            )?["items"]
                .as_array()
                .ok_or("Failed to get playlists")?
                .iter()
                .for_each(|playlist| {
                    println!(
                        "{}",
                        Playlist::from_id(playlist["id"].as_str().unwrap(), acc).unwrap()
                    )
                });
            Ok(())
        }
        "search" => {
            check_args_len(&args, 2)?;
            let query = args[2..].join(" ");
            let account = this.config.get_an_account();
            if account.is_none() {
                return Err("No accounts found. At least one is required to use the API. Try adding one with 'adduser'".parse().unwrap());
            }
            info!("Searching for {}. This may take a few moments...", args[1]);
            match ContentType::from_str(args[1]) {
                Some(typ) => {
                    match typ {
                        // generic hell
                        ContentType::Tracks => spotify_api_search::<Track>(
                            query.as_str(),
                            &typ,
                            account.ok_or("No account")?,
                        )?
                        .iter()
                        .for_each(|x| println!("{}", x)),
                        ContentType::Albums => spotify_api_search::<Album>(
                            query.as_str(),
                            &typ,
                            account.ok_or("No account")?,
                        )?
                        .iter()
                        .for_each(|x| println!("{}", x)),
                        ContentType::Artists => spotify_api_search::<Artist>(
                            query.as_str(),
                            &typ,
                            account.ok_or("No account")?,
                        )?
                        .iter()
                        .for_each(|x| println!("{}", x)),
                        ContentType::Playlists => spotify_api_search::<Playlist>(
                            query.as_str(),
                            &typ,
                            account.ok_or("No account")?,
                        )?
                        .iter()
                        .for_each(|x| println!("{}", x)),
                    }
                    Ok(())
                }
                None => Err(
                    "Invalid content type. Valid types are: 'track', 'album', 'artist', 'playlist'"
                        .parse()
                        .unwrap(),
                ),
            }
        }
        "config" => {
            println!("config file is {:?}", this.file);
            Ok(())
        }
        "users" => {
            println!("Found {} users:", this.config.accounts.len());
            for (key, acc) in this.config.accounts.iter_mut() {
                let mut id = String::from(acc.get_token()?);
                id.truncate(20);
                id.push_str("...");
                println!("{}: {}", key, id);
            }
            Ok(())
        }
        "delusers" => {
            if user_yn(
                "Are you sure you want to delete ALL accounts? This cannot be undone!",
                false,
            ) {
                this.config.accounts.clear();
                this.config.save_to(&mut this.file)?;
                info!("deleted all users.");
                Ok(())
            } else {
                Ok(())
            }
        }
        _ => Err(format!("Unknown command: {}", args[0])),
    }
}

fn user_yn(prompt: &str, default: bool) -> bool {
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

fn user_choose<T: Display + Clone>(prompt: &str, data: Vec<T>, default: usize) -> T {
    let mut i: u16 = 0;
    for t in data {
        println!("[{}]: {}", i, t);
        i += 1;
    }
    let mut input = String::new();
    print!("{} (default: {}): ", prompt, default);
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input).unwrap();
    let input = input
        .trim()
        .parse::<usize>()
        .unwrap_or_else(|_| default as usize);
    data.get(input).unwrap().clone()
}

fn check_args_len(args: &Vec<&str>, len: usize) -> Result<(), String> {
    if args.len() < len + 1 {
        return Err(format!(
            "Not enough arguments. Expected {}, got {}.",
            len,
            args.len() - 1
        ));
    }
    Ok(())
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
