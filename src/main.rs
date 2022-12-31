use std::fs::File;
use std::io;
use std::io::Write;

use colored::Colorize;
//use reqwest::blocking::Client;

use crate::account::Account;
use crate::config::{load, Config};

mod account;
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
            exit(0)
        }
        "add" => {
            if check_args_len(&args, 1) {
                this.config
                    .add_account(&mut this.file, args[1], Account::new());
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

//fn request_api(account: &Account, endpoint: &str, typ: reqwest::Method) -> String {
//    let base = "https://api.spotify.com/v1";
  //  String::from("L")
    //let response = Client::new().request(typ, format!("{}{}", base, endpoint)).json();
//}

fn exit(code: i8) {
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
    exit(-1);
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
