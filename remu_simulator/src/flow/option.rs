use std::collections::{BTreeMap, BTreeSet};

use remu_state::StateOption;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendArgKv {
    key: String,
    value: String,
}

impl BackendArgKv {
    #[inline]
    fn new(key: String, value: String) -> Self {
        Self { key, value }
    }
}

fn is_valid_key_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

fn parse_backend_arg_kv(raw: &str) -> Result<BackendArgKv, String> {
    let (key_raw, value_raw) = raw.split_once('=').ok_or_else(|| {
        format!("invalid --sim-opt {raw:?}: expected KEY=VALUE (e.g. watchdog=5)")
    })?;

    let key = key_raw.trim();
    let value = value_raw.trim();
    if key.is_empty() {
        return Err(format!("invalid --sim-opt {raw:?}: KEY cannot be empty"));
    }
    if value.is_empty() {
        return Err(format!("invalid --sim-opt {raw:?}: VALUE cannot be empty"));
    }
    if !key.chars().all(is_valid_key_char) {
        return Err(format!(
            "invalid --sim-opt key {key:?}: use only [A-Za-z0-9_-]"
        ));
    }
    Ok(BackendArgKv::new(key.to_string(), value.to_string()))
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BackendArgs {
    map: BTreeMap<String, String>,
}

impl BackendArgs {
    pub fn try_from_kv(kv: &[BackendArgKv]) -> Result<Self, String> {
        let mut map = BTreeMap::new();
        for item in kv {
            if let Some(old) = map.insert(item.key.clone(), item.value.clone()) {
                return Err(format!(
                    "duplicate --sim-opt key {:?}: previous value {:?}, new value {:?}",
                    item.key, old, item.value
                ));
            }
        }
        Ok(Self { map })
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(String::as_str)
    }

    pub fn assert_known_keys(&self, known: &[&str]) -> Result<(), String> {
        let known: BTreeSet<&str> = known.iter().copied().collect();
        let unknown: Vec<String> = self
            .map
            .keys()
            .filter(|k| !known.contains(k.as_str()))
            .cloned()
            .collect();
        if unknown.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "unsupported --sim-opt key(s): {}",
                unknown.join(", ")
            ))
        }
    }
}

#[derive(clap::Args, Debug, Clone)]
pub struct SimulatorOption {
    /// State Option
    #[command(flatten)]
    pub state: StateOption,

    /// Backend-specific simulator option in KEY=VALUE form (e.g. watchdog=5).
    /// Repeat this flag to set multiple options, or pass multiple KEY=VALUE pairs space-separated.
    #[arg(long = "sim-opt", value_name = "KEY=VALUE", value_parser = parse_backend_arg_kv, num_args = 1..)]
    pub sim_opt: Vec<BackendArgKv>,
}

impl SimulatorOption {
    pub fn backend_args(&self) -> Result<BackendArgs, String> {
        BackendArgs::try_from_kv(&self.sim_opt)
    }
}
