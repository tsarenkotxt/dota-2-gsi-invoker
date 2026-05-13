use std::time::Instant;

const READY_THRESHOLD_SECONDS: f32 = 0.05;
const REFRESH_DETECTION_SECONDS: f32 = 0.5;
const REFRESH_DETECTION_COUNT: usize = 2;

pub const CANONICAL_SPELLS: [InvokerSpell; 10] = [
    InvokerSpell::new("cold_snap", "Cold_Snap.webp"),
    InvokerSpell::new("chaos_meteor", "Chaos_Meteor.webp"),
    InvokerSpell::new("deafening_blast", "Deafening_Blast.webp"),
    InvokerSpell::new("ice_wall", "Ice_Wall.webp"),
    InvokerSpell::new("ghost_walk", "Ghost_Walk.webp"),
    InvokerSpell::new("sun_strike", "Sun_Strike.webp"),
    InvokerSpell::new("emp", "EMP.webp"),
    InvokerSpell::new("alacrity", "Alacrity.webp"),
    InvokerSpell::new("forge_spirit", "Forge_Spirit.webp"),
    InvokerSpell::new("tornado", "Tornado.webp"),
];

#[derive(Clone, Copy, Debug)]
pub struct InvokerSpell {
    pub id: &'static str,
    pub asset: &'static str,
}

impl InvokerSpell {
    pub const fn new(id: &'static str, asset: &'static str) -> Self {
        Self { id, asset }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SpellSnapshot {
    pub cooldown_remaining: f32,
    pub cooldown_total: f32,
    pub mana_cost: Option<u16>,
    pub known: bool,
}

#[derive(Clone, Debug)]
pub struct CooldownUpdate {
    pub is_invoker: bool,
    pub is_hero_demo: bool,
    pub paused: bool,
    pub current_mana: Option<f32>,
    pub spells: [SpellSnapshot; 10],
}

#[derive(Clone, Debug)]
pub struct CooldownState {
    is_invoker: bool,
    paused: bool,
    pause_started: Option<Instant>,
    current_mana: Option<f32>,
    last_update: Option<Instant>,
    spells: [TrackedSpell; 10],
}

impl CooldownState {
    pub fn new() -> Self {
        Self {
            is_invoker: false,
            paused: false,
            pause_started: None,
            current_mana: None,
            last_update: None,
            spells: [TrackedSpell::default(); 10],
        }
    }

    pub fn apply(&mut self, update: CooldownUpdate) {
        let now = Instant::now();
        self.apply_pause_state(update.paused, now);
        self.is_invoker = update.is_invoker;
        if !self.paused {
            self.current_mana = update.current_mana;
        }
        self.last_update = Some(now);

        if !self.paused {
            if self.detect_external_refresh(&update, now) {
                for spell in &mut self.spells {
                    spell.cooldown_until = None;
                }
            }

            for (tracked, incoming) in self.spells.iter_mut().zip(update.spells) {
                if incoming.known {
                    tracked.known = true;
                    tracked.mana_cost = incoming.mana_cost.or(tracked.mana_cost);
                    let previous_remaining = tracked
                        .cooldown_until
                        .map(|until| until.saturating_duration_since(now).as_secs_f32())
                        .unwrap_or_default();

                    if incoming.cooldown_remaining > READY_THRESHOLD_SECONDS {
                        if tracked.cooldown_total <= READY_THRESHOLD_SECONDS
                            || incoming.cooldown_remaining
                                > previous_remaining + REFRESH_DETECTION_SECONDS
                        {
                            tracked.cooldown_total = incoming.cooldown_remaining;
                        }

                        tracked.cooldown_until = Some(
                            now + std::time::Duration::from_secs_f32(incoming.cooldown_remaining),
                        );
                    } else {
                        tracked.cooldown_until = None;
                        tracked.cooldown_total = 0.0;
                    }
                }
            }
        }
    }

    pub fn snapshot(&self) -> OverlaySnapshot {
        let now = self.pause_started.unwrap_or_else(Instant::now);
        let spells = self.spells.map(|spell| {
            let remaining = spell
                .cooldown_until
                .map(|until| until.saturating_duration_since(now).as_secs_f32())
                .unwrap_or_default();

            SpellSnapshot {
                cooldown_remaining: remaining,
                cooldown_total: spell.cooldown_total,
                mana_cost: spell.mana_cost,
                known: spell.known,
            }
        });

        OverlaySnapshot {
            is_invoker: self.is_invoker,
            paused: self.paused,
            connected: self
                .last_update
                .is_some_and(|last| last.elapsed().as_secs_f32() < 3.0),
            current_mana: self.current_mana,
            spells,
        }
    }

    fn apply_pause_state(&mut self, paused: bool, now: Instant) {
        match (self.paused, paused) {
            (false, true) => {
                self.paused = true;
                self.pause_started = Some(now);
            }
            (true, false) => {
                if let Some(started) = self.pause_started.take() {
                    let pause_duration = now.saturating_duration_since(started);
                    for spell in &mut self.spells {
                        if let Some(until) = spell.cooldown_until {
                            spell.cooldown_until = Some(until + pause_duration);
                        }
                    }
                }
                self.paused = false;
            }
            _ => {}
        }
    }

    fn detect_external_refresh(&self, update: &CooldownUpdate, now: Instant) -> bool {
        if !(update.is_invoker && update.is_hero_demo) {
            return false;
        }

        self.spells
            .iter()
            .zip(update.spells)
            .filter(|(tracked, incoming)| {
                incoming.known
                    && incoming.cooldown_remaining <= READY_THRESHOLD_SECONDS
                    && tracked.cooldown_until.is_some_and(|until| {
                        until.saturating_duration_since(now).as_secs_f32()
                            > REFRESH_DETECTION_SECONDS
                    })
            })
            .count()
            >= REFRESH_DETECTION_COUNT
    }
}

#[derive(Clone, Debug)]
pub struct OverlaySnapshot {
    pub is_invoker: bool,
    pub paused: bool,
    pub connected: bool,
    pub current_mana: Option<f32>,
    pub spells: [SpellSnapshot; 10],
}

#[derive(Clone, Copy, Debug, Default)]
struct TrackedSpell {
    known: bool,
    mana_cost: Option<u16>,
    cooldown_until: Option<Instant>,
    cooldown_total: f32,
}

pub fn spell_index(raw_name: &str) -> Option<usize> {
    let normalized = normalize_ability_name(raw_name);
    CANONICAL_SPELLS
        .iter()
        .position(|spell| spell.id == normalized)
}

pub fn spell_by_id(id: &str) -> Option<&'static InvokerSpell> {
    CANONICAL_SPELLS.iter().find(|spell| spell.id == id)
}

pub fn normalize_ability_name(raw_name: &str) -> &str {
    raw_name
        .strip_prefix("invoker_")
        .or_else(|| raw_name.strip_prefix("special_bonus_unique_invoker_"))
        .unwrap_or(raw_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn resume_extends_cooldowns_by_pause_duration() {
        let now = Instant::now();
        let mut state = CooldownState::new();
        state.spells[0].cooldown_until = Some(now + Duration::from_secs(10));
        state.paused = true;
        state.pause_started = Some(now);

        state.apply_pause_state(false, now + Duration::from_secs(4));

        let cooldown_until = state.spells[0].cooldown_until.unwrap();
        assert_eq!(
            cooldown_until.saturating_duration_since(now),
            Duration::from_secs(14)
        );
        assert!(!state.paused);
        assert!(state.pause_started.is_none());
    }

    #[test]
    fn external_refresh_clears_all_tracked_cooldowns() {
        let now = Instant::now();
        let mut state = CooldownState::new();
        state.is_invoker = true;
        state.spells[0].known = true;
        state.spells[0].cooldown_until = Some(now + Duration::from_secs(10));
        state.spells[1].known = true;
        state.spells[1].cooldown_until = Some(now + Duration::from_secs(12));
        state.spells[2].known = true;
        state.spells[2].cooldown_until = Some(now + Duration::from_secs(14));

        let mut update = CooldownUpdate {
            is_invoker: true,
            is_hero_demo: true,
            paused: false,
            current_mana: Some(300.0),
            spells: [SpellSnapshot::default(); 10],
        };
        update.spells[0] = SpellSnapshot {
            cooldown_remaining: 0.0,
            cooldown_total: 0.0,
            mana_cost: None,
            known: true,
        };
        update.spells[1] = SpellSnapshot {
            cooldown_remaining: 0.0,
            cooldown_total: 0.0,
            mana_cost: None,
            known: true,
        };

        state.apply(update);

        assert!(
            state
                .spells
                .iter()
                .all(|spell| spell.cooldown_until.is_none())
        );
    }

    #[test]
    fn single_ready_spell_does_not_clear_other_cooldowns() {
        let now = Instant::now();
        let mut state = CooldownState::new();
        state.is_invoker = true;
        state.spells[0].known = true;
        state.spells[0].cooldown_until = Some(now + Duration::from_secs(10));
        state.spells[1].known = true;
        state.spells[1].cooldown_until = Some(now + Duration::from_secs(12));
        state.spells[2].known = true;
        state.spells[2].cooldown_until = Some(now + Duration::from_secs(14));

        let mut update = CooldownUpdate {
            is_invoker: true,
            is_hero_demo: true,
            paused: false,
            current_mana: Some(300.0),
            spells: [SpellSnapshot::default(); 10],
        };
        update.spells[0] = SpellSnapshot {
            cooldown_remaining: 0.0,
            cooldown_total: 0.0,
            mana_cost: None,
            known: true,
        };

        state.apply(update);

        assert!(state.spells[0].cooldown_until.is_none());
        assert!(state.spells[1].cooldown_until.is_some());
    }

    #[test]
    fn cooldown_total_is_kept_while_counting_down() {
        let mut state = CooldownState::new();
        let mut update = CooldownUpdate {
            is_invoker: true,
            is_hero_demo: true,
            paused: false,
            current_mana: Some(300.0),
            spells: [SpellSnapshot::default(); 10],
        };
        update.spells[0] = SpellSnapshot {
            cooldown_remaining: 10.0,
            cooldown_total: 0.0,
            mana_cost: None,
            known: true,
        };

        state.apply(update.clone());
        update.spells[0].cooldown_remaining = 8.0;
        state.apply(update);

        assert_eq!(state.spells[0].cooldown_total, 10.0);
    }

    #[test]
    fn external_refresh_is_ignored_outside_hero_demo() {
        let now = Instant::now();
        let mut state = CooldownState::new();
        state.is_invoker = true;
        state.spells[0].known = true;
        state.spells[0].cooldown_until = Some(now + Duration::from_secs(10));
        state.spells[1].known = true;
        state.spells[1].cooldown_until = Some(now + Duration::from_secs(12));
        state.spells[2].known = true;
        state.spells[2].cooldown_until = Some(now + Duration::from_secs(14));

        let mut update = CooldownUpdate {
            is_invoker: true,
            is_hero_demo: false,
            paused: false,
            current_mana: Some(300.0),
            spells: [SpellSnapshot::default(); 10],
        };
        update.spells[0] = SpellSnapshot {
            cooldown_remaining: 0.0,
            cooldown_total: 0.0,
            mana_cost: None,
            known: true,
        };
        update.spells[1] = SpellSnapshot {
            cooldown_remaining: 0.0,
            cooldown_total: 0.0,
            mana_cost: None,
            known: true,
        };

        state.apply(update);

        assert!(state.spells[0].cooldown_until.is_none());
        assert!(state.spells[1].cooldown_until.is_none());
        assert!(state.spells[2].cooldown_until.is_some());
    }
}
