use crate::library::Song;
use crate::state::{ActiveTab, AppState};
use crate::stats::format::{format_duration, unknown_if_empty};
use crate::stats::model::{StatsRange, StatsSummary, TopEntry};
use crate::theme::{self, icons, radius, space, text};
use crate::ui::textures::TextureCache;
use crate::ui::widgets::bar_chart::bar_chart;
use crate::ui::widgets::chip::chip;
use crate::ui::widgets::cover::{paint_cover, skeleton};
use crate::ui::widgets::empty_state::empty_state;
use crate::ui::widgets::glass::{glass_panel, GlassKind};
use eframe::egui::{self, RichText};

const GAP: f32 = 16.0;
const ROW_H: f32 = 52.0;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, textures: &mut TextureCache) {
    header(ui, state);

    let handle = state.stats.handle().clone();
    if let Some(reason) = handle.disabled_reason() {
        empty_state(ui, icons::CHART_BAR, "Estadísticas no disponibles", &reason, None);
        return;
    }

    // Se vuelve a pedir el resumen solo si cambió el rango o llegó un evento nuevo.
    let key = (state.stats_range, handle.events_version());
    if state.stats_requested != Some(key) {
        handle.request_summary(state.stats_range);
        state.stats_requested = Some(key);
    }

    let range = state.stats_range;
    let Some(summary) = handle.latest_summary().filter(|(r, _)| *r == range).map(|(_, s)| s) else {
        loading(ui);
        return;
    };

    if summary.totals.plays == 0 {
        let go = empty_state(
            ui,
            icons::CHART_BAR,
            "Sin reproducciones en este rango",
            "Reproduce una canción más de 5 segundos para empezar.",
            Some("Ir a Canciones"),
        );
        if go {
            state.select_tab(ActiveTab::All);
        }
        return;
    }

    let songs = &state.songs;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        tiles(ui, &summary);
        ui.add_space(GAP);
        timeline_card(ui, &summary);
        ui.add_space(GAP);
        habits_card(ui, &summary);
        ui.add_space(GAP);
        tops(ui, &summary, songs, textures);
        ui.add_space(GAP);
    });
}

fn header(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("ESTADÍSTICAS").font(theme::deco(text::XL + 4.0)).color(theme::accent(ui.ctx())));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for range in StatsRange::ALL_RANGES.iter().rev() {
                if chip(ui, range.label(), state.stats_range == *range).clicked() {
                    state.stats_range = *range;
                }
            }
        });
    });
    ui.add_space(space::LG);
}

fn loading(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        for _ in 0..4 {
            skeleton(ui, 96.0, radius::MD);
        }
    });
}

fn tiles(ui: &mut egui::Ui, summary: &StatsSummary) {
    // El marco suma margen y borde alrededor del contenido: se descuentan para que los cuatro quepan.
    let width = ((ui.available_width() - 3.0 * GAP) / 4.0 - 2.0).max(140.0);
    let items = [
        ("Tiempo total", format_duration(summary.totals.listened_ms)),
        ("Reproducciones", summary.totals.plays.to_string()),
        ("Canciones únicas", summary.totals.unique_songs.to_string()),
        ("Promedio diario", format_duration(summary.avg_daily_ms)),
    ];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        for (label, value) in items {
            glass_panel(ui, GlassKind::Standard, radius::LG, space::LG, |ui| {
                ui.set_width(width - 2.0 * space::LG);
                ui.vertical(|ui| {
                    ui.label(RichText::new(label).size(text::SM).color(theme::TEXT_MUTED));
                    ui.label(RichText::new(value).font(theme::bold(text::XL + 4.0)));
                });
            });
        }
    });
}

fn timeline_card(ui: &mut egui::Ui, summary: &StatsSummary) {
    glass_panel(ui, GlassKind::Standard, radius::LG, space::XL, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("LÍNEA DE TIEMPO").font(theme::deco(text::LG)).color(theme::accent(ui.ctx())));
        if let Some(peak) = summary.peak_bucket.and_then(|i| summary.timeline.get(i)) {
            ui.label(
                RichText::new(format!("Pico: {} · {}", peak.label, format_duration(peak.listened_ms)))
                    .size(text::SM)
                    .color(theme::TEXT_MUTED),
            );
        }
        ui.add_space(space::MD);
        bar_chart(ui, &summary.timeline, summary.peak_bucket, 180.0);
    });
}

fn habits_card(ui: &mut egui::Ui, summary: &StatsSummary) {
    let h = &summary.habits;
    let peak_day = h.peak_weekday.as_ref().map(|(day, ms)| format!("{day} ({})", format_duration(*ms)));
    let metrics = [
        ("Días activos", h.active_days.to_string()),
        ("Racha actual", format!("{} d", h.current_streak_days)),
        ("Racha más larga", format!("{} d", h.longest_streak_days)),
        ("Sesiones", h.sessions.to_string()),
        ("Sesión promedio", format_duration(h.avg_session_ms)),
        ("Sesión más larga", format_duration(h.longest_session_ms)),
        ("Sesiones por día", format!("{:.1}", h.sessions_per_day)),
        ("Día pico", peak_day.unwrap_or_else(|| "—".to_string())),
    ];
    glass_panel(ui, GlassKind::Standard, radius::LG, space::XL, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("HÁBITOS").font(theme::deco(text::LG)).color(theme::accent(ui.ctx())));
        ui.add_space(space::MD);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(space::XXL, space::LG);
            for (label, value) in metrics {
                ui.vertical(|ui| {
                    ui.label(RichText::new(label).size(text::SM).color(theme::TEXT_MUTED));
                    ui.label(RichText::new(value).font(theme::bold(text::MD)));
                });
            }
        });
    });
}

fn tops(ui: &mut egui::Ui, summary: &StatsSummary, songs: &[Song], textures: &mut TextureCache) {
    ui.columns(3, |cols| {
        top_card(&mut cols[0], "CANCIONES", &summary.top_songs, songs, textures, icons::MUSIC_NOTES, "Sin título", false);
        top_card(&mut cols[1], "ARTISTAS", &summary.top_artists, songs, textures, icons::USER, "Artista desconocido", true);
        top_card(&mut cols[2], "ÁLBUMES", &summary.top_albums, songs, textures, icons::DISC, "Álbum desconocido", false);
    });
}

#[allow(clippy::too_many_arguments)]
fn top_card(
    ui: &mut egui::Ui,
    title: &str,
    entries: &[TopEntry],
    songs: &[Song],
    textures: &mut TextureCache,
    fallback_icon: &str,
    unknown: &str,
    round_cover: bool,
) {
    glass_panel(ui, GlassKind::Standard, radius::LG, space::LG, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(title).font(theme::deco(text::LG)).color(theme::accent(ui.ctx())));
        ui.add_space(space::SM);
        let max = entries.first().map(|e| e.listened_ms).unwrap_or(1).max(1);
        for (i, entry) in entries.iter().enumerate() {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_H), egui::Sense::hover());
            let cover_rect = egui::Rect::from_center_size(egui::pos2(rect.left() + 38.0, rect.center().y - 3.0), egui::vec2(36.0, 36.0));
            let cover = songs.iter().find(|s| s.path == entry.reference_path).and_then(|s| s.cover_art.clone());
            paint_cover(ui, cover_rect, textures, &cover, if round_cover { 18.0 } else { 6.0 }, fallback_icon, false);

            let accent = theme::accent(ui.ctx());
            let painter = ui.painter();
            painter.text(
                egui::pos2(rect.left() + 8.0, rect.center().y - 3.0),
                egui::Align2::LEFT_CENTER,
                (i + 1).to_string(),
                theme::bold(text::SM),
                theme::TEXT_MUTED,
            );
            let text_left = cover_rect.right() + 10.0;
            let clip = painter.with_clip_rect(egui::Rect::from_min_max(
                egui::pos2(text_left, rect.top()),
                egui::pos2(rect.right() - 70.0, rect.bottom()),
            ));
            clip.text(
                egui::pos2(text_left, rect.center().y - 12.0),
                egui::Align2::LEFT_CENTER,
                unknown_if_empty(&entry.name, unknown),
                theme::bold(text::BASE),
                theme::TEXT_MAIN,
            );
            let detail = if entry.detail.is_empty() {
                format!("{} rep.", entry.plays)
            } else {
                format!("{} · {} rep.", unknown_if_empty(&entry.detail, "Artista desconocido"), entry.plays)
            };
            clip.text(
                egui::pos2(text_left, rect.center().y + 4.0),
                egui::Align2::LEFT_CENTER,
                detail,
                egui::FontId::proportional(text::XS),
                theme::TEXT_MUTED,
            );
            painter.text(
                egui::pos2(rect.right() - 6.0, rect.center().y - 12.0),
                egui::Align2::RIGHT_CENTER,
                format_duration(entry.listened_ms),
                egui::FontId::proportional(text::SM),
                theme::TEXT_MAIN,
            );
            let bar_y = rect.bottom() - 4.0;
            let full = rect.width() - 8.0;
            let filled = full * (entry.listened_ms as f32 / max as f32);
            painter.rect_filled(egui::Rect::from_min_size(egui::pos2(rect.left() + 4.0, bar_y), egui::vec2(full, 3.0)), 1.5, egui::Color32::from_white_alpha(22));
            painter.rect_filled(egui::Rect::from_min_size(egui::pos2(rect.left() + 4.0, bar_y), egui::vec2(filled, 3.0)), 1.5, accent);
        }
    });
}
