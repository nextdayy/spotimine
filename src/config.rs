use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::account::Account;
use crate::utils::Pair;
use crate::{info, Spotimine};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub(crate) accounts: HashMap<String, Account>,
}

impl Config {
    pub(crate) fn init(path: &Path) -> Result<Pair<File, Config>, String> {
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
            let config = Config {
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
        let config: Config =
            serde_json::from_str(string).map_err(|e| format!("Error deserializing: {}", e))?;
        Ok(config)
    }

    pub fn save_to(&self, file: &mut File) -> Result<(), String> {
        file.set_len(0).map_err(|e| e.to_string())?;
        file.write_all(
            serde_json::to_string_pretty(self)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
        .map_err(|e| e.to_string())
    }

    /// Returns the account with the given alias, and saves the config
    pub(crate) fn add_account(
        &mut self,
        file: &mut File,
        key: &str,
        acc: Account,
    ) -> Result<(), String> {
        info!("Adding account named {}", key);
        self.accounts.insert(String::from(key), acc);
        self.save_to(file)
    }

    /// Returns the account with the given alias, and saves the config
    pub(crate) fn remove_account(&mut self, file: &mut File, key: &str) -> Result<(), String> {
        info!("Adding account named {}", key);
        self.accounts.remove(key);
        self.save_to(file)
    }

    pub(crate) fn get_account(&mut self, key: &str) -> Option<&mut Account> {
        let acc = self.accounts.get_mut(key);
        acc
    }

    pub(crate) fn get_an_account(&mut self) -> Option<&mut Account> {
        let acc = self.accounts.iter_mut().next().map(|(_, v)| v);
        acc
    }
}

pub(crate) fn load() -> Result<Spotimine, String> {
    return match std::env::consts::OS {
        "windows" => {
            let path = format!("{}\\spotimine", std::env::var("APPDATA").unwrap());
            std::fs::create_dir_all(&path).expect("Failed to create config directory");
            Spotimine::new(format!("{}\\config.json", path))
        }
        "linux" | "android" => {
            let path = format!("{}/.config/spotimine", std::env::var("HOME").unwrap());
            std::fs::create_dir_all(&path).expect("Failed to create config directory");
            Spotimine::new(format!("{}/config.json", path))
        }
        _ => Err(format!("{} is not supported.", std::env::consts::OS)),
    };
}
