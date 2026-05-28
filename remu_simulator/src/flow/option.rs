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

fn is_valid_key_segment(seg: &str) -> bool {
    !seg.is_empty()
        && seg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn parse_backend_arg_kv(raw: &str) -> Result<BackendArgKv, String> {
    let (key_raw, value_raw) = raw.split_once('=').ok_or_else(|| {
        format!("invalid --sim-opt {raw:?}: expected KEY=VALUE (e.g. nzea.target=tile)")
    })?;

    let key = key_raw.trim();
    let value = value_raw.trim();
    if key.is_empty() {
        return Err(format!("invalid --sim-opt {raw:?}: KEY cannot be empty"));
    }
    if value.is_empty() {
        return Err(format!("invalid --sim-opt {raw:?}: VALUE cannot be empty"));
    }
    if !key.contains('.') {
        return Err(format!(
            "invalid --sim-opt key {key:?}: key must be namespaced (e.g. nzea.target)"
        ));
    }
    if key.starts_with('.') || key.ends_with('.') || key.contains("..") {
        return Err(format!(
            "invalid --sim-opt key {key:?}: malformed namespace path"
        ));
    }
    if key.split('.').any(|seg| !is_valid_key_segment(seg)) {
        return Err(format!(
            "invalid --sim-opt key {key:?}: use only [A-Za-z0-9_-] in each segment"
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

    pub fn namespaces(&self) -> BTreeSet<String> {
        self.map
            .keys()
            .filter_map(|k| k.split('.').next())
            .map(str::to_string)
            .collect()
    }

    pub fn assert_only_namespaces(&self, allowed: &[&str]) -> Result<(), String> {
        let allowed: BTreeSet<&str> = allowed.iter().copied().collect();
        let unknown: Vec<String> = self
            .namespaces()
            .into_iter()
            .filter(|ns| !allowed.contains(ns.as_str()))
            .collect();
        if unknown.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "unsupported --sim-opt namespace(s): {}",
                unknown.join(", ")
            ))
        }
    }

    pub fn scope<'a>(&'a self, ns: &'a str) -> BackendScope<'a> {
        BackendScope { ns, map: &self.map }
    }
}

pub struct BackendScope<'a> {
    ns: &'a str,
    map: &'a BTreeMap<String, String>,
}

impl<'a> BackendScope<'a> {
    pub fn get(&self, key: &str) -> Option<&'a str> {
        let full = format!("{}.{}", self.ns, key);
        self.map.get(&full).map(String::as_str)
    }

    pub fn keys(&self) -> Vec<String> {
        let prefix = format!("{}.", self.ns);
        self.map
            .keys()
            .filter_map(|k| k.strip_prefix(&prefix))
            .map(str::to_string)
            .collect()
    }

    pub fn assert_known_keys(&self, known: &[&str]) -> Result<(), String> {
        let known: BTreeSet<&str> = known.iter().copied().collect();
        let unknown: Vec<String> = self
            .keys()
            .into_iter()
            .filter(|k| !known.contains(k.as_str()))
            .collect();
        if unknown.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "unsupported --sim-opt key(s) under {}: {}",
                self.ns,
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

    /// Backend-specific simulator option in KEY=VALUE form (namespaced, e.g. nzea.target=tile).
    /// Repeat this flag to set multiple options.
    #[arg(long = "sim-opt", value_name = "KEY=VALUE", value_parser = parse_backend_arg_kv)]
    pub sim_opt: Vec<BackendArgKv>,
}

impl SimulatorOption {
    pub fn backend_args(&self) -> Result<BackendArgs, String> {
        BackendArgs::try_from_kv(&self.sim_opt)
    }
}

#[cfg(test)]
mod tests {
    use super::{BackendArgKv, BackendArgs, parse_backend_arg_kv};

    #[test]
    fn parse_valid_backend_arg() {
        let kv = parse_backend_arg_kv("nzea.target=tile").unwrap();
        assert_eq!(
            kv,
            BackendArgKv {
                key: "nzea.target".to_string(),
                value: "tile".to_string()
            }
        );
    }

    #[test]
    fn parse_requires_namespace() {
        let err = parse_backend_arg_kv("target=tile").unwrap_err();
        assert!(err.contains("namespaced"));
    }

    #[test]
    fn duplicate_key_rejected() {
        let kv0 = parse_backend_arg_kv("nzea.target=core").unwrap();
        let kv1 = parse_backend_arg_kv("nzea.target=tile").unwrap();
        let err = BackendArgs::try_from_kv(&[kv0, kv1]).unwrap_err();
        assert!(err.contains("duplicate"));
    }

    #[test]
    fn scope_lookup_works() {
        let kv0 = parse_backend_arg_kv("nzea.target=tile").unwrap();
        let kv1 = parse_backend_arg_kv("nzea.trace=1").unwrap();
        let args = BackendArgs::try_from_kv(&[kv0, kv1]).unwrap();
        let nzea = args.scope("nzea");
        assert_eq!(nzea.get("target"), Some("tile"));
        assert_eq!(nzea.get("trace"), Some("1"));
        assert_eq!(nzea.get("missing"), None);
    }
}
