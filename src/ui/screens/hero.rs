use crate::theme::{self, icons, radius, space, text, with_alpha};
use crate::ui::textures::TextureCache;
use crate::ui::widgets::cover::paint_cover;
use crate::ui::widgets::glass::{paint_glass, GlassKind};
use crate::ui::widgets::gradient::radial_glow_mesh;
use crate::ui::widgets::pill_button::{PillButton, PillKind};
use eframe::egui::{self, RichText};

pub struct HeroInfo {
    pub eyebrow: String,
    pub title: String,
    pub subtitle: String,
    pub cover: Option<String>,
    /// Carátula circular (artistas) en vez de cuadrada.
    pub circle: bool,
}

#[derive(PartialEq)]
pub enum HeroAction {
    None,
    Play,
    Shuffle,
}

/// Por debajo de este alto disponible el encabezado se compacta.
const COMPACT_BELOW: f32 = 560.0;

/// Encabezado de la pantalla: carátula, título, resumen y acciones.
pub fn show(ui: &mut egui::Ui, textures: &mut TextureCache, info: &HeroInfo) -> HeroAction {
    let accent = theme::accent(ui.ctx());
    let compact = ui.available_height() < COMPACT_BELOW;
    let (height, cover_side) = if compact { (164.0, 112.0) } else { (196.0, 144.0) };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::hover());
    let painter = ui.painter();
    paint_glass(painter, rect, GlassKind::Standard, radius::XL, 0.0);
    painter.rect_filled(rect, radius::XL, with_alpha(accent, 34));

    let cover_rect = egui::Rect::from_min_size(egui::pos2(rect.left() + 26.0, rect.center().y - cover_side / 2.0), egui::vec2(cover_side, cover_side));
    painter
        .with_clip_rect(rect)
        .add(radial_glow_mesh(cover_rect.center(), 150.0, with_alpha(accent, 70), 40));
    let rounding = if info.circle { cover_side / 2.0 } else { radius::MD };
    paint_cover(ui, cover_rect, textures, &info.cover, rounding, icons::MUSIC_NOTES, true);

    let text_rect = egui::Rect::from_min_max(
        egui::pos2(cover_rect.right() + 28.0, rect.top() + 24.0),
        egui::pos2(rect.right() - 26.0, rect.bottom() - 24.0),
    );
    let mut action = HeroAction::None;
    let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(text_rect).layout(egui::Layout::top_down(egui::Align::Min)));
    if !compact {
        inner.label(RichText::new(&info.eyebrow).font(theme::bold(text::XS)).color(accent));
    }
    let title_size = if compact { text::XXL } else { text::XXL + 8.0 };
    inner.add(egui::Label::new(RichText::new(&info.title).font(theme::deco(title_size)).color(theme::TEXT_MAIN)).truncate());
    inner.label(RichText::new(&info.subtitle).color(theme::TEXT_MUTED));
    inner.add_space(space::MD);
    inner.horizontal(|ui| {
        if PillButton::new("Reproducir", PillKind::Primary).icon(icons::PLAY).show(ui).clicked() {
            action = HeroAction::Play;
        }
        if PillButton::new("Aleatorio", PillKind::Secondary).icon(icons::SHUFFLE).show(ui).clicked() {
            action = HeroAction::Shuffle;
        }
    });
    action
}
