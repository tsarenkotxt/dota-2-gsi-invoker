use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, bail};
use serde::Deserialize;

use crate::invoker::{CANONICAL_SPELLS, spell_by_id};

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub gsi_port: u16,
    pub overlay_x: f32,
    pub overlay_y: f32,
    pub debug_gsi: bool,
    pub show_footer_row: bool,
    pub skill_order: Vec<DisplaySkill>,
}

#[derive(Clone, Debug)]
pub struct DisplaySkill {
    pub id: &'static str,
    pub asset: &'static str,
    pub mana_cost: u16,
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let path = Path::new("dota_2_gsi_invoker_config.json");
        if !path.exists() {
            return Ok(Self::default());
        }

        let raw = std::fs::read_to_string(path)?;
        let file: ConfigFile = serde_json::from_str(&raw)?;
        file.into_app_config()
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        let mana_cost = default_mana_cost();
        let skill_order = default_skill_order(&mana_cost).unwrap_or_else(|err| {
            panic!("default skill config is invalid: {err:#}");
        });

        Self {
            gsi_port: 53000,
            debug_gsi: false,
            show_footer_row: true,
            overlay_x: -1.0,
            overlay_y: -1.0,
            skill_order,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    gsi_port: Option<u16>,
    overlay_x: Option<f32>,
    overlay_y: Option<f32>,
    debug_gsi: Option<bool>,
    show_footer_row: Option<bool>,
    skill_mana_cost: Option<HashMap<String, u16>>,
    skill_order: Option<Vec<String>>,
}

impl ConfigFile {
    fn into_app_config(self) -> anyhow::Result<AppConfig> {
        let defaults = AppConfig::default();
        let mana_cost = self.skill_mana_cost.unwrap_or_else(default_mana_cost);
        let skill_ids = self.skill_order.unwrap_or_else(default_skill_ids);

        Ok(AppConfig {
            gsi_port: self.gsi_port.unwrap_or(defaults.gsi_port),
            overlay_x: self.overlay_x.unwrap_or(defaults.overlay_x),
            overlay_y: self.overlay_y.unwrap_or(defaults.overlay_y),
            debug_gsi: self.debug_gsi.unwrap_or(defaults.debug_gsi),
            show_footer_row: self.show_footer_row.unwrap_or(defaults.show_footer_row),
            skill_order: resolve_skill_order(&skill_ids, &mana_cost)
                .context("invalid skill_order in dota_2_gsi_invoker_config.json")?,
        })
    }
}

fn default_mana_cost() -> HashMap<String, u16> {
    [
        ("cold_snap", 100),
        ("chaos_meteor", 200),
        ("deafening_blast", 250),
        ("ice_wall", 125),
        ("ghost_walk", 175),
        ("sun_strike", 175),
        ("emp", 125),
        ("alacrity", 75),
        ("forge_spirit", 75),
        ("tornado", 140),
    ]
    .into_iter()
    .map(|(spell, cost)| (spell.to_owned(), cost))
    .collect()
}

fn default_skill_ids() -> Vec<String> {
    CANONICAL_SPELLS
        .iter()
        .map(|spell| spell.id.to_owned())
        .collect()
}

fn default_skill_order(mana_cost: &HashMap<String, u16>) -> anyhow::Result<Vec<DisplaySkill>> {
    resolve_skill_order(&default_skill_ids(), mana_cost)
}

fn resolve_skill_order(
    skill_ids: &[String],
    mana_cost: &HashMap<String, u16>,
) -> anyhow::Result<Vec<DisplaySkill>> {
    if skill_ids.len() != 10 {
        bail!(
            "skill_order must contain exactly 10 skills, found {}",
            skill_ids.len()
        );
    }

    let mut seen = std::collections::HashSet::new();
    let mut skills = Vec::with_capacity(10);

    for id in skill_ids {
        if !seen.insert(id.as_str()) {
            bail!("skill_order contains duplicate skill '{id}'");
        }

        let Some(spell) = spell_by_id(id) else {
            bail!("unknown skill '{id}'");
        };
        let Some(&cost) = mana_cost.get(spell.id) else {
            bail!("skill_mana_cost is missing value for skill '{}'", spell.id);
        };

        skills.push(DisplaySkill {
            id: spell.id,
            asset: spell.asset,
            mana_cost: cost,
        });
    }

    Ok(skills)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_ten_resolved_skills() {
        let config = AppConfig::default();

        assert_eq!(config.skill_order.len(), 10);
        assert_eq!(config.skill_order[0].id, "cold_snap");
    }

    #[test]
    fn rejects_duplicate_skill_order_entries() {
        let mut skill_ids = default_skill_ids();
        skill_ids[1] = skill_ids[0].clone();

        let err = resolve_skill_order(&skill_ids, &default_mana_cost()).unwrap_err();

        assert!(err.to_string().contains("duplicate"));
    }
}
