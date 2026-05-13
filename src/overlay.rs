use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, TAU};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use display_info::DisplayInfo;
use eframe::egui::{
    self, Align2, Color32, ColorImage, CornerRadius, FontId, Mesh, Pos2, Rect, Stroke,
    TextureHandle, TextureOptions, Vec2, ViewportBuilder, WindowLevel,
};
use parking_lot::RwLock;

use crate::config::AppConfig;
use crate::embedded_assets;
use crate::invoker::{CooldownState, spell_index};

const TILE: f32 = 46.0;
const GAP: f32 = 5.0;
const PAD: f32 = 8.0;
const WIDTH: f32 = PAD * 2.0 + TILE * 5.0 + GAP * 4.0;
const SKILL_ROWS: f32 = 2.0;

pub fn run(state: Arc<RwLock<CooldownState>>, config: AppConfig) -> eframe::Result {
    let height = overlay_height(&config);
    let mut viewport = ViewportBuilder::default()
        .with_title("dota_2_gsi_invoker")
        .with_inner_size([WIDTH, height])
        .with_resizable(false)
        .with_decorations(false)
        .with_transparent(true)
        .with_window_level(WindowLevel::AlwaysOnTop);

    if let Some(position) = overlay_position(&config) {
        viewport = viewport.with_position(position);
    }
    #[cfg(target_os = "windows")]
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let app = InvokerOverlay::new(state, config);
    eframe::run_native(
        "dota_2_gsi_invoker",
        options,
        Box::new(|creation| {
            configure_style(&creation.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}

struct InvokerOverlay {
    state: Arc<RwLock<CooldownState>>,
    textures: HashMap<&'static str, TextureHandle>,
    config: AppConfig,
    #[cfg(target_os = "windows")]
    mouse_passthrough_enabled: bool,
}

impl InvokerOverlay {
    fn new(state: Arc<RwLock<CooldownState>>, config: AppConfig) -> Self {
        Self {
            state,
            textures: HashMap::new(),
            config,
            #[cfg(target_os = "windows")]
            mouse_passthrough_enabled: false,
        }
    }

    fn ensure_textures(&mut self, ctx: &egui::Context) {
        for skill in &self.config.skill_order {
            if self.textures.contains_key(skill.id) {
                continue;
            }

            match load_spell_image(skill.asset) {
                Ok(image) => {
                    self.textures.insert(
                        skill.id,
                        ctx.load_texture(skill.id, image, TextureOptions::LINEAR),
                    );
                }
                Err(err) => eprintln!("failed to load {}: {err:#}", skill.asset),
            }
        }
    }
}

impl eframe::App for InvokerOverlay {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ensure_textures(ctx);
        self.configure_mouse_passthrough(ctx);

        let snapshot = self.state.read().snapshot();
        if !(snapshot.connected && snapshot.is_invoker) {
            ctx.request_repaint_after(Duration::from_millis(250));
            return;
        }

        ctx.request_repaint_after(Duration::from_millis(33));
        let alpha = if snapshot.paused { 200 } else { 238 };

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let painter = ui.painter();
                let panel =
                    Rect::from_min_size(Pos2::ZERO, Vec2::new(WIDTH, overlay_height(&self.config)));
                painter.rect_filled(
                    panel,
                    CornerRadius::same(7),
                    Color32::from_rgba_unmultiplied(18, 20, 24, alpha),
                );
                painter.rect_stroke(
                    panel.shrink(0.5),
                    CornerRadius::same(7),
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(188, 154, 83, 190)),
                    egui::StrokeKind::Inside,
                );

                if self.config.show_footer_row {
                    draw_footer_row(painter);
                }

                for (index, skill) in self.config.skill_order.iter().enumerate() {
                    let Some(spell_index) = spell_index(skill.id) else {
                        continue;
                    };
                    let col = index % 5;
                    let row = index / 5;
                    let min = Pos2::new(
                        PAD + col as f32 * (TILE + GAP),
                        PAD + row as f32 * (TILE + GAP),
                    );
                    let rect = Rect::from_min_size(min, Vec2::splat(TILE));
                    let spell_state = snapshot.spells[spell_index];
                    let mana_cost = spell_state.mana_cost.unwrap_or(skill.mana_cost);
                    let missing_mana = snapshot.connected
                        && snapshot.is_invoker
                        && snapshot
                            .current_mana
                            .zip(Some(mana_cost))
                            .is_some_and(|(mana, cost)| mana + 0.5 < cost as f32);

                    painter.rect_filled(rect, CornerRadius::same(4), Color32::from_rgb(10, 11, 13));

                    if let Some(texture) = self.textures.get(skill.id) {
                        painter.image(
                            texture.id(),
                            rect.shrink(2.0),
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            if missing_mana {
                                Color32::from_rgb(128, 146, 178)
                            } else {
                                Color32::WHITE
                            },
                        );
                    }

                    if missing_mana {
                        draw_missing_mana_overlay(painter, rect.shrink(2.0), 104);
                    }

                    let cooldown = cooldown_label(spell_state.cooldown_remaining);
                    if cooldown.is_some() {
                        draw_cooldown_overlay(
                            painter,
                            rect.shrink(2.0),
                            spell_state.cooldown_remaining,
                            spell_state.cooldown_total,
                        );
                        if missing_mana {
                            draw_missing_mana_overlay(painter, rect.shrink(2.0), 64);
                        }
                    }

                    if mana_cost > 0 {
                        draw_mana_cost(painter, rect, mana_cost, missing_mana);
                    }

                    painter.rect_stroke(
                        rect,
                        CornerRadius::same(4),
                        Stroke::new(1.0, Color32::from_rgba_unmultiplied(212, 170, 93, 150)),
                        egui::StrokeKind::Inside,
                    );

                    if let Some(cooldown) = cooldown {
                        draw_cooldown_text(painter, rect, cooldown);
                    }
                }
            });
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }
}

impl InvokerOverlay {
    #[cfg(target_os = "windows")]
    fn configure_mouse_passthrough(&mut self, ctx: &egui::Context) {
        if self.mouse_passthrough_enabled {
            return;
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(true));
        self.mouse_passthrough_enabled = true;
    }

    #[cfg(not(target_os = "windows"))]
    fn configure_mouse_passthrough(&mut self, _ctx: &egui::Context) {}
}

fn configure_style(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = Color32::TRANSPARENT;
    visuals.panel_fill = Color32::TRANSPARENT;
    ctx.set_visuals(visuals);
}

fn overlay_height(config: &AppConfig) -> f32 {
    let rows = if config.show_footer_row {
        SKILL_ROWS + 1.0
    } else {
        SKILL_ROWS
    };

    PAD * 2.0 + TILE * rows + GAP * (rows - 1.0)
}

fn draw_footer_row(painter: &egui::Painter) {
    let top = PAD + SKILL_ROWS * TILE + SKILL_ROWS * GAP;
    let rect = Rect::from_min_size(Pos2::new(PAD, top), Vec2::new(WIDTH - PAD * 2.0, TILE));

    painter.rect_filled(
        rect,
        CornerRadius::same(4),
        Color32::from_rgba_unmultiplied(10, 11, 13, 120),
    );
}

fn cooldown_label(seconds: f32) -> Option<String> {
    let rounded = seconds.round() as u32;
    (rounded > 0).then(|| rounded.to_string())
}

fn cooldown_text_pos(rect: Rect) -> Pos2 {
    rect.center() - Vec2::new(0.0, 3.0)
}

fn draw_cooldown_text(painter: &egui::Painter, rect: Rect, label: String) {
    let pos = cooldown_text_pos(rect);
    let font_id = FontId::proportional(19.0);

    painter.text(
        pos + Vec2::new(1.0, 1.0),
        Align2::CENTER_CENTER,
        &label,
        font_id.clone(),
        Color32::from_rgba_unmultiplied(0, 0, 0, 210),
    );
    painter.text(pos, Align2::CENTER_CENTER, label, font_id, Color32::WHITE);
}

fn draw_cooldown_overlay(painter: &egui::Painter, rect: Rect, remaining: f32, total: f32) {
    painter.rect_filled(
        rect,
        CornerRadius::same(3),
        Color32::from_rgba_unmultiplied(0, 0, 0, 92),
    );

    let fraction = if total > 0.0 {
        (remaining / total).clamp(0.0, 1.0)
    } else {
        1.0
    };

    if fraction >= 0.99 {
        painter.rect_filled(
            rect,
            CornerRadius::same(3),
            Color32::from_rgba_unmultiplied(0, 0, 0, 176),
        );
        return;
    }

    if fraction <= 0.01 {
        return;
    }

    let clipped = painter.with_clip_rect(rect);
    let center = rect.center();
    let radius = rect.size().length() * 0.56;
    let segments = ((48.0 * fraction).ceil() as usize).clamp(2, 48);
    let start_angle = -FRAC_PI_2;
    let sweep = -TAU * fraction;
    let color = Color32::from_rgba_unmultiplied(0, 0, 0, 185);
    let mut mesh = Mesh::default();

    for segment in 0..segments {
        let a0 = start_angle + sweep * segment as f32 / segments as f32;
        let a1 = start_angle + sweep * (segment + 1) as f32 / segments as f32;
        let base = mesh.vertices.len() as u32;

        mesh.colored_vertex(center, color);
        mesh.colored_vertex(
            center + Vec2::new(a0.cos() * radius, a0.sin() * radius),
            color,
        );
        mesh.colored_vertex(
            center + Vec2::new(a1.cos() * radius, a1.sin() * radius),
            color,
        );
        mesh.add_triangle(base, base + 1, base + 2);
    }

    clipped.add(mesh);
}

fn draw_missing_mana_overlay(painter: &egui::Painter, rect: Rect, alpha: u8) {
    painter.rect_filled(
        rect,
        CornerRadius::same(3),
        Color32::from_rgba_unmultiplied(16, 42, 90, alpha),
    );
}

fn draw_mana_cost(painter: &egui::Painter, rect: Rect, mana_cost: u16, missing_mana: bool) {
    let label = mana_cost.to_string();
    let pos = Pos2::new(rect.right() - 4.0, rect.bottom() - 3.0);
    let font_id = FontId::proportional(10.5);
    let galley = painter.layout_no_wrap(label.clone(), font_id.clone(), Color32::WHITE);
    let background_width = galley.size().x + 6.0;
    let background = Rect::from_min_max(
        Pos2::new(
            pos.x - background_width - 1.0,
            pos.y - galley.size().y - 2.0,
        ),
        Pos2::new(pos.x + 2.0, pos.y + 1.0),
    );
    let text_pos = background.center();
    let color = if missing_mana {
        Color32::from_rgb(104, 168, 255)
    } else {
        Color32::from_rgb(96, 190, 255)
    };

    painter.rect_filled(
        background,
        CornerRadius {
            nw: 2,
            ne: 0,
            sw: 0,
            se: 0,
        },
        Color32::from_rgba_unmultiplied(0, 0, 0, 190),
    );
    painter.text(
        text_pos + Vec2::new(1.0, 1.0),
        Align2::CENTER_CENTER,
        &label,
        font_id.clone(),
        Color32::from_rgba_unmultiplied(0, 0, 0, 210),
    );
    painter.text(text_pos, Align2::CENTER_CENTER, label, font_id, color);
}

fn load_spell_image(asset_name: &str) -> anyhow::Result<ColorImage> {
    if let Some(bytes) = embedded_assets::spell_image(asset_name) {
        return color_image_from_bytes(bytes);
    }

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(asset_name);
    let bytes = std::fs::read(path)?;
    color_image_from_bytes(&bytes)
}

fn color_image_from_bytes(bytes: &[u8]) -> anyhow::Result<ColorImage> {
    let rgba = image::load_from_memory(bytes)?.into_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];

    Ok(ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}

#[cfg(target_os = "windows")]
fn window_icon() -> Option<egui::IconData> {
    let rgba = image::load_from_memory(embedded_assets::spell_image("Ghost_Walk.webp")?)
        .ok()?
        .into_rgba8();
    let (width, height) = rgba.dimensions();

    Some(egui::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    })
}

fn overlay_position(config: &AppConfig) -> Option<Pos2> {
    if config.overlay_x >= 0.0 && config.overlay_y >= 0.0 {
        return Some(Pos2::new(config.overlay_x, config.overlay_y));
    }

    let display = DisplayInfo::all()
        .ok()?
        .into_iter()
        .find(|display| display.is_primary)
        .or_else(|| DisplayInfo::all().ok()?.into_iter().next())?;

    let scale = display.scale_factor.max(1.0);
    let width = display.width as f32 / scale;
    let height = display.height as f32 / scale;
    let x = display.x as f32 / scale + width - WIDTH - 168.0;
    let y = display.y as f32 / scale + height - overlay_height(config) - 92.0;

    Some(Pos2::new(x.max(0.0), y.max(0.0)))
}
