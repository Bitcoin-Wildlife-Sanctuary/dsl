use std::collections::HashMap;
use anyhow::{Error, Result};

#[derive(Clone)]
pub struct Options {
    pub map: HashMap<String, OptionsEntry>
}

#[derive(Clone)]
pub enum OptionsEntry {
    String(String),
    Binary(Vec<u8>),
    MultiBinary(Vec<Vec<u8>>),
    U32(u32),
    MultiU32(Vec<u32>),
    U64(u64),
    MultiU64(Vec<u64>),
}

impl Options {
    pub fn new() -> Options {
        Options {
            map: HashMap::new()
        }
    }

    pub fn with_entry(mut self, name: impl ToString, entry: OptionsEntry) -> Options {
        self.map.insert(name.to_string(), entry);
        self
    }

    pub fn with_string(mut self, name: impl ToString, entry: impl ToString) -> Options {
        self.map.insert(name.to_string(), OptionsEntry::String(entry.to_string()));
        self
    }

    pub fn with_binary(mut self, name: impl ToString, entry: Vec<u8>) -> Options {
        self.map.insert(name.to_string(), OptionsEntry::Binary(entry));
        self
    }

    pub fn with_multi_binary(mut self, name: impl ToString, entry: Vec<Vec<u8>>) -> Options {
        self.map.insert(name.to_string(), OptionsEntry::MultiBinary(entry));
        self
    }

    pub fn with_u32(mut self, name: impl ToString, entry: u32) -> Options {
        self.map.insert(name.to_string(), OptionsEntry::U32(entry));
        self
    }

    pub fn with_multi_u32(mut self, name: impl ToString, entry: Vec<u32>) -> Options {
        self.map.insert(name.to_string(), OptionsEntry::MultiU32(entry));
        self
    }

    pub fn with_u64(mut self, name: impl ToString, entry: u64) -> Options {
        self.map.insert(name.to_string(), OptionsEntry::U64(entry));
        self
    }

    pub fn with_multi_u64(mut self, name: impl ToString, entry: Vec<u64>) -> Options {
        self.map.insert(name.to_string(), OptionsEntry::MultiU64(entry));
        self
    }

    pub fn exists(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }

    pub fn get_string(&self, name: &impl ToString) -> Result<&String> {
        match self.map.get(&name.to_string()) {
            Some(OptionsEntry::String(v)) => Ok(v),
            _ => Err(Error::msg("The corresponding option must be a string"))
        }
    }

    pub fn get_binary(&self, name: &impl ToString) -> Result<&[u8]> {
        match self.map.get(&name.to_string()) {
            Some(OptionsEntry::Binary(v)) => Ok(v),
            _ => Err(Error::msg("The corresponding option must be a binary"))
        }
    }

    pub fn get_multi_binary(&self, name: &impl ToString) -> Result<&[Vec<u8>]> {
        match self.map.get(&name.to_string()) {
            Some(OptionsEntry::MultiBinary(v)) => Ok(v),
            _ => Err(Error::msg("The corresponding option must be a multi binary"))
        }
    }

    pub fn get_u32(&self, name: &impl ToString) -> Result<u32> {
        match self.map.get(&name.to_string()) {
            Some(OptionsEntry::U32(v)) => Ok(*v),
            _ => Err(Error::msg("The corresponding option must be a u32"))
        }
    }

    pub fn get_multi_u32(&self, name: &impl ToString) -> Result<&[u32]> {
        match self.map.get(&name.to_string()) {
            Some(OptionsEntry::MultiU32(v)) => Ok(v),
            _ => Err(Error::msg("The corresponding option must be a multi u32"))
        }
    }

    pub fn get_u64(&self, name: &impl ToString) -> Result<u64> {
        match self.map.get(&name.to_string()) {
            Some(OptionsEntry::U64(v)) => Ok(*v),
            _ => Err(Error::msg("The corresponding option must be a u64"))
        }
    }

    pub fn get_multi_u64(&self, name: &impl ToString) -> Result<&[u64]> {
        match self.map.get(&name.to_string()) {
            Some(OptionsEntry::MultiU64(v)) => Ok(v),
            _ => Err(Error::msg("The corresponding option must be a multi u64"))
        }
    }
}

