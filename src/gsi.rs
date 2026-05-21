use serde_json::Value;

use crate::invoker::{CooldownUpdate, SpellSnapshot, spell_index};

const DOTA_PRE_GAME: &str = "DOTA_GAMERULES_STATE_PRE_GAME";
const DOTA_GAME_IN_PROGRESS: &str = "DOTA_GAMERULES_STATE_GAME_IN_PROGRESS";

pub fn parse_invoker_cooldowns(payload: &[u8]) -> anyhow::Result<CooldownUpdate> {
    let value: Value = serde_json::from_slice(payload)?;
    let is_invoker = value
        .pointer("/hero/name")
        .and_then(Value::as_str)
        .is_some_and(|name| name == "npc_dota_hero_invoker");
    let is_play_active = string_at(&value, &["/map/game_state"]).is_some_and(is_play_active_state);
    let paused = bool_at(&value, &["/map/paused"]).unwrap_or_default();
    let is_hero_demo = string_at(&value, &["/map/name"])
        .is_some_and(|name| name == "hero_demo_main")
        || string_at(&value, &["/map/customgamename", "/map/custom_game_name"])
            .is_some_and(|name| name.contains("dota_addons/hero_demo"));
    let current_mana = number_at(&value, &["/player/mana", "/hero/mana"]);

    let mut spells = [SpellSnapshot::default(); 10];

    if let Some(abilities) = value.get("abilities").and_then(Value::as_object) {
        for ability in abilities.values() {
            let Some(name) = ability.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(index) = spell_index(name) else {
                continue;
            };

            let cooldown = number_field(
                ability,
                &["cooldown", "cooldown_remaining", "cooldown_time"],
            )
            .unwrap_or_default()
            .max(0.0);
            let level = number_field(ability, &["level"]).unwrap_or_default();
            let mana_cost = number_field(
                ability,
                &["mana_cost", "manacost", "mana_cost_current", "mana"],
            )
            .map(|mana| mana.round().max(0.0) as u16);
            spells[index] = SpellSnapshot {
                cooldown_remaining: cooldown,
                cooldown_total: 0.0,
                mana_cost,
                known: level > 0.0 || cooldown > 0.0,
            };
        }
    }

    Ok(CooldownUpdate {
        is_invoker,
        is_play_active,
        is_hero_demo,
        paused,
        current_mana,
        spells,
    })
}

fn string_at<'a>(value: &'a Value, pointers: &[&str]) -> Option<&'a str> {
    pointers
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(Value::as_str))
}

fn is_play_active_state(game_state: &str) -> bool {
    matches!(game_state, DOTA_PRE_GAME | DOTA_GAME_IN_PROGRESS)
}

fn bool_at(value: &Value, pointers: &[&str]) -> Option<bool> {
    pointers.iter().find_map(|pointer| {
        value.pointer(pointer).and_then(|candidate| {
            candidate.as_bool().or_else(|| {
                candidate.as_str().and_then(|text| match text {
                    "true" | "1" => Some(true),
                    "false" | "0" => Some(false),
                    _ => None,
                })
            })
        })
    })
}

fn number_at(value: &Value, pointers: &[&str]) -> Option<f32> {
    pointers.iter().find_map(|pointer| {
        value.pointer(pointer).and_then(|candidate| {
            candidate
                .as_f64()
                .or_else(|| candidate.as_str()?.parse::<f64>().ok())
                .map(|number| number as f32)
        })
    })
}

fn number_field(value: &Value, fields: &[&str]) -> Option<f32> {
    fields.iter().find_map(|field| {
        value.get(*field).and_then(|candidate| {
            candidate
                .as_f64()
                .or_else(|| candidate.as_str()?.parse::<f64>().ok())
                .map(|number| number as f32)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_invoker_ability_cooldowns() {
        let payload = br#"{
            "hero": {"name": "npc_dota_hero_invoker"},
            "map": {
                "name": "hero_demo_main",
                "game_state": "DOTA_GAMERULES_STATE_GAME_IN_PROGRESS",
                "paused": true,
                "customgamename": "common/dota 2 beta/game/dota_addons/hero_demo"
            },
            "player": {"mana": 120},
            "abilities": {
                "ability0": {"name": "invoker_cold_snap", "level": 1, "cooldown": 17.5},
                "ability1": {"name": "invoker_tornado", "level": 4, "cooldown": 0, "mana_cost": 140}
            }
        }"#;

        let update = parse_invoker_cooldowns(payload).unwrap();

        assert!(update.is_invoker);
        assert!(update.is_play_active);
        assert!(update.is_hero_demo);
        assert!(update.paused);
        assert_eq!(update.current_mana, Some(120.0));
        assert_eq!(update.spells[0].cooldown_remaining, 17.5);
        assert_eq!(update.spells[0].cooldown_total, 0.0);
        let tornado = spell_index("invoker_tornado").unwrap();
        assert_eq!(update.spells[tornado].mana_cost, Some(140));
        assert!(update.spells[tornado].known);
    }

    #[test]
    fn detects_non_demo_map() {
        let payload = br#"{
            "hero": {"name": "npc_dota_hero_invoker"},
            "map": {"name": "start", "paused": false},
            "abilities": {}
        }"#;

        let update = parse_invoker_cooldowns(payload).unwrap();

        assert!(update.is_invoker);
        assert!(!update.is_hero_demo);
    }

    #[test]
    fn detects_pre_game_as_play_active() {
        let payload = br#"{
            "hero": {"name": "npc_dota_hero_invoker"},
            "map": {
                "name": "start",
                "game_state": "DOTA_GAMERULES_STATE_PRE_GAME",
                "paused": false
            },
            "abilities": {}
        }"#;

        let update = parse_invoker_cooldowns(payload).unwrap();

        assert!(update.is_invoker);
        assert!(update.is_play_active);
    }

    #[test]
    fn detects_hero_selection_as_not_play_active() {
        let payload = br#"{
            "hero": {"name": "npc_dota_hero_invoker"},
            "map": {
                "name": "start",
                "game_state": "DOTA_GAMERULES_STATE_HERO_SELECTION",
                "paused": false
            },
            "abilities": {}
        }"#;

        let update = parse_invoker_cooldowns(payload).unwrap();

        assert!(update.is_invoker);
        assert!(!update.is_play_active);
    }
}
