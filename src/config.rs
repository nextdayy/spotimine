use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::account::Account;
use crate::utils::Pair;
use crate::Spotimine;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub(crate) accounts: HashMap<String, Account>,
}

impl Config {
    pub(crate) fn init(path: &str) -> Result<Pair<File, Config>, String> {
        let path = Path::new(path);
        let mut file: File;
        if path.exists() {
            file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
                .map_err(|e| e.to_string())?;
            let cfg = Config::load(&mut file)?;
            Ok(Pair { a: file, b: cfg })
        } else {
            file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(path)
                .map_err(|e| e.to_string())?;
            let mut config = Config {
                accounts: HashMap::new(),
            };
            config.save_to(&mut file)?;
            Ok(Pair { a: file, b: config })
        }
    }

    fn load(file: &mut File) -> Result<Config, String> {
        let mut read = String::new();
        BufReader::new(file)
            .read_to_string(&mut read)
            .map_err(|e| e.to_string())?;
        let string = read.as_str().trim().trim_matches(char::from(0));
        let config: Config = serde_json::from_str(string)
            .map_err(|e| format!("Error deserializing: {}", e))?;
        Ok(config)
    }

    fn save_to(&mut self, file: &mut File) -> Result<(), String> {
        file.set_len(0).expect("Failed to clear config file");
        file
            .write_all(
                serde_json::to_string(self)
                    .map_err(|e| e.to_string())?
                    .as_bytes(),
            )
            .map_err(|e| e.to_string())
    }

    /// Returns the account with the given alias, and saves the config
    pub(crate) fn add_account(&mut self, file: &mut File, key: &str, acc: Account) {
        self.accounts.insert(String::from(key), acc);
        let _ = self.save_to(file);
    }
}

pub(crate) fn load() -> Result<Spotimine, String> {
    return match std::env::consts::OS {
        "windows" => {
            let path = format!("{}\\spotimine", std::env::var("APPDATA").unwrap());
            std::fs::create_dir_all(&path).expect("Failed to create config directory");
            Spotimine::new(format!("{}\\config.json", path))
        }
        "linux" => {
            let path = format!("{}/.config/spotimine", std::env::var("HOME").unwrap());
            std::fs::create_dir_all(&path).expect("Failed to create config directory");
            Spotimine::new(format!("{}/config.json", path))
        }
        _ => Err(format!("{} is not supported.", std::env::consts::OS)),
    };
}

/*fn do_aliases(&mut self, json: String) {
    let mut i: usize = 0;
    for aliaseses in json.split("\"aliases\":[").collect::<Vec<&str>>()[1..].iter() {
        let aliases;
        if aliaseses.trim().ends_with("}") {
            aliases = &aliaseses[..aliaseses.len() - 6];
        } else {
            aliases = &aliaseses[..aliaseses.len() - 1];
        }
        for alias in aliases.split(",").collect::<Vec<&str>>() {
            self.accounts[i].aliases.push(alias.replace("\"", ""));
        }
        i += 1;
    }
}*/

/*pub(crate) fn save(&mut self) {
    let mut result = String::from("{ \"accounts\": [");
    for acc in &self.accounts {
        result.push_str(&acc.to_json());
        result.push_str(", ");
    }
    if result.ends_with(",") {
        result.pop();
    }
    result.push_str("] }");
    let _ = self.file.set_len(0);
    match self.file.write_all(result.as_bytes()) {
        Ok(_) => {
            info!("Created/Saved config");
        }
        Err(e) => {
            error!("Failed to save config: {}", e);
        }
    }
}

fn load(file: File) -> Result<Config, String> {
    let mut result = String::new();
    let mut reader = BufReader::new(&file);
    reader
        .read_to_string(&mut result)
        .map_err(|e| format!("failed to read config file: {}", e.to_string()))?;
    let mut accounts = Vec::new();
    for acc in result.split("},").collect::<Vec<&str>>().iter() {
        accounts.push(from_json(acc.to_string())?);
    }
    info!("Loaded config ({} accounts)", accounts.len());
    let mut config = Config { accounts, file };
    let _ = config.do_aliases(result);
    Ok(config)
}*/
