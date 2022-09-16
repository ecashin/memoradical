use anyhow::{anyhow, Context, Result};
use gloo_storage::{LocalStorage, Storage};
use sha2::{Digest, Sha256};

pub struct LocalStore {
    checksum: String,
    key: String,
    value: String,
}

impl LocalStore {
    pub fn new(key: &str, default_data: &str) -> Result<Self> {
        let key = key.to_string();
        let data = match LocalStorage::get(&key) {
            Ok(data) => data,
            Err(_) => {
                LocalStorage::set(&key, default_data.to_string())?;
                default_data.to_string()
            }
        };
        let checksum = Self::hash(&data);
        Ok(Self {
            checksum,
            key,
            value: data,
        })
    }
    pub fn value(&self) -> String {
        self.value.clone()
    }
    fn hash(value: &str) -> String {
        let mut hasher = Sha256::new();

        hasher.update(value.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    pub fn save(&mut self, value: &str) -> Result<()> {
        let data: String = LocalStorage::get(&self.key)?;
        if Self::hash(&data) == self.checksum {
            LocalStorage::set(&self.key, value.to_string()).context("storing local data")?;
            self.value = value.to_string();
            Ok(())
        } else {
            Err(anyhow!("local storage has been changed since last loaded"))
        }
    }
}
