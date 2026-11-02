#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::OnceLock;

use serde_yaml::{Mapping, Value};

static REFERENCE_CONFIG: OnceLock<PathBuf> = OnceLock::new();

/// Build a temporary copy of the shipped config that matches the canonical
/// reference model: no cross-hand alt rules and no hand restrictions.
///
/// The reference scores and audit rulings are calibrated for the same-hand
/// model.  User experiments in `config.yaml` (cross-hand rules, generation
/// restrictions) must not change what those regression tests measure.
pub fn reference_config() -> PathBuf {
    REFERENCE_CONFIG
        .get_or_init(|| {
            let dir = std::env::temp_dir().join(format!("shirolyzer-tests-{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.join("config.reference.yaml");

            let source = shirolyzer::paths::config_dir().join("config.yaml");
            let text = std::fs::read_to_string(&source).unwrap();
            let mut value: Value = serde_yaml::from_str(&text).unwrap();
            if let Some(root) = value.as_mapping_mut() {
                if let Some(rules) = root.get_mut("alt_fingers").and_then(Value::as_sequence_mut) {
                    rules.retain(|rule| {
                        let Some(map) = rule.as_mapping() else {
                            return true;
                        };
                        let ngram = map.get("ngram").and_then(Value::as_sequence);
                        let fingers = map.get("fingers").and_then(Value::as_sequence);
                        let (Some(ngram), Some(fingers)) = (ngram, fingers) else {
                            return true;
                        };
                        if ngram.len() != fingers.len() {
                            return true;
                        }
                        ngram.iter().zip(fingers).all(|(key, finger)| {
                            let Some(key) = key.as_str().and_then(|s| s.chars().next()) else {
                                return true;
                            };
                            let Some(finger) = finger
                                .as_str()
                                .and_then(shirolyzer::geometry::parse_finger8)
                            else {
                                return true;
                            };
                            shirolyzer::geometry::finger_hand(finger)
                                == shirolyzer::geometry::hand_of(&key.to_string())
                        })
                    });
                }

                let mut restrictions = Mapping::new();
                restrictions.insert(Value::String("left".into()), Value::Sequence(Vec::new()));
                restrictions.insert(Value::String("right".into()), Value::Sequence(Vec::new()));
                root.insert(
                    Value::String("hand_restrictions".into()),
                    Value::Mapping(restrictions),
                );
            }

            std::fs::write(&path, serde_yaml::to_string(&value).unwrap()).unwrap();
            path
        })
        .clone()
}
