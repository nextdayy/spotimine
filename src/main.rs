extern crate core;

use core::str;
use std::fmt::Display;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{io, thread};

use crossterm::style::Stylize;
use signal_hook::consts::SIGINT;

use crate::account::Account;
use crate::api::{do_api_json, get_liked_songs, get_playlists_for, spotify_api_search};
use crate::config::{load, Config};
use crate::data::{Album, Artist, ContentType, Playlist, Track};

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
            .underlined()
    );
    let mut this = load().expect("Failed to initialize");
    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(SIGINT, Arc::clone(&term))
        .expect("Failed to register signal handler");
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term))
        .expect("Failed to register signal handler");
    signal_hook::flag::register(signal_hook::consts::SIGQUIT, Arc::clone(&term))
        .expect("Failed to register signal handler");

    while !term.load(Ordering::Relaxed) {
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
    exit(1, &mut this);
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
                match do_api_json("GET", "me", &mut account, "") {
                    Ok(res) => this.config.add_account(
                        &mut this.file,
                        res["display_name"]
                            .as_str()
                            .ok_or("Failed to get display name")?,
                        account,
                    ),
                    Err(_) => Err("Failed to get account info from Spotify API."
                        .parse()
                        .unwrap()),
                }
            } else {
                check_args_len(&args, 1, "adduser [<optional> alias]")?;
                this.config
                    .add_account(&mut this.file, args[1], Account::new()?)
            }
        }
        "rmuser" => {
            check_args_len(&args, 1, "rmuser [account_name]")?;
            this.config.remove_account(&mut this.file, args[1])
        }
        "liked" => {
            check_args_len(&args, 1, "liked [account_name]")?;
            let account = this.config.get_account(args[1]).ok_or("Unknown Account")?;
            let mut liked = get_liked_songs(account)?;
            liked.print_tracks_ordered();
            Ok(())
        }
        "copy" => {
            check_args_len(&args, 2, "copy [source account] [dst account] [<optional> target_name, use liked to OVERWRITE liked songs]")?;
            let acc = this.config.get_account(args[1]);
            if acc.is_none() {
                return Err(format!(
                    "Account not found: {}. Try adding one with 'adduser'",
                    args[1]
                ));
            }
            let acc = &mut acc.unwrap().clone();
            let acc2 = this.config.get_account(args[2]);
            let target_name = args.get(3).copied();
            if acc2.is_none() {
                return Err(format!(
                    "Account not found: {}. Try adding one with 'adduser'",
                    args[2]
                ));
            }
            let acc2 = &mut acc2.unwrap();
            let mut vec = get_playlists_for(acc)?;
            vec.push(get_liked_songs(acc)?);
            let p = user_choose("Choose a playlist to copy", vec, 0)?;
            if args.get(3).copied().unwrap_or_default() == "liked" {
                p.copy_to_liked(acc2)?;
            } else {
                p.copy(acc, target_name, Some(acc2))?;
            }
            Ok(())
        }
        "search" => {
            check_args_len(&args, 2, "search [content_type] [query...]")?;
            let query = args[2..].join(" ");
            let account = this.config.get_an_account();
            if account.is_none() {
                return Err("No accounts found. At least one is required to use the API. Try adding one with 'adduser'".parse().unwrap());
            }
            let account = account.ok_or("No account")?;
            info!("Searching for {}. This may take a few moments...", args[1]);
            match ContentType::from_str(args[1]) {
                Some(typ) => {
                    match typ {
                        // generic hell
                        ContentType::Tracks => {
                            spotify_api_search::<Track>(query.as_str(), &typ, account)?
                                .iter()
                                .for_each(|x| println!("{}", x))
                        }
                        ContentType::Albums => {
                            spotify_api_search::<Album>(query.as_str(), &typ, account)?
                                .iter()
                                .for_each(|x| println!("{}", x))
                        }
                        ContentType::Artists => {
                            spotify_api_search::<Artist>(query.as_str(), &typ, account)?
                                .iter()
                                .for_each(|x| println!("{}", x))
                        }
                        ContentType::Playlists => {
                            spotify_api_search::<Playlist>(query.as_str(), &typ, account)?
                                .iter()
                                .for_each(|x| println!("{}", x))
                        }
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

fn user_choose<T: Display + Clone>(
    prompt: &str,
    data: Vec<T>,
    default: usize,
) -> Result<T, String> {
    for (i, t) in (0_u16..).zip(data.iter()) {
        println!("[{}]: {}", i, t);
    }
    let mut input = String::new();
    print!("{} (default: {}): ", prompt, default);
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim().parse::<usize>().map_err(|_| "Invalid input")?;
    Ok(data.get(input).unwrap().clone())
}

fn user_choose_multi<T: Display + Clone>(prompt: &str, data: Vec<T>) -> Result<Vec<T>, String> {
    for (i, t) in (0_u16..).zip(data.iter()) {
        println!("[{}]: {}", i, t);
    }
    let mut input = String::new();
    print!("{} (eg: '1 2 3', '3-6'): ", prompt);
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input).unwrap();
    let mut out = Vec::new();
    let _ = input
        .trim()
        .split(' ')
        .try_for_each(|v| -> Result<(), String> {
            if v.contains('-') {
                let mut split = v.split('-');
                let start = split
                    .next()
                    .unwrap()
                    .parse::<usize>()
                    .map_err(|_| "Invalid input")?;
                let end = split
                    .next()
                    .unwrap()
                    .parse::<usize>()
                    .map_err(|_| "Invalid input")?;
                (start..=end).for_each(|i| out.push(data[i].clone()));
                Ok(())
            } else {
                out.push(data[v.parse::<usize>().map_err(|_| "Invalid input")?].clone());
                Ok(())
            }
        });
    Ok(out)
}

fn wait(prompt: &str, time_s: u8) {
    let mut time = time_s;
    while time > 0 {
        print!("{} ({}s) ", prompt, time);
        io::stdout().flush().unwrap();
        thread::sleep(Duration::from_secs(1));
        print!("\r");
        io::stdout().flush().unwrap();
        time -= 1;
    }
}

fn check_args_len(args: &Vec<&str>, len: usize, help: &str) -> Result<(), String> {
    if args.len() < len + 1 {
        return Err(format!(
            "Not enough arguments. Expected {}, got {}.\nUsage: {}",
            len,
            args.len() - 1,
            help
        ));
    }
    Ok(())
}

fn exit(code: i8, this: &mut Spotimine) {
    println!("{}", "Exiting...".red());
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
