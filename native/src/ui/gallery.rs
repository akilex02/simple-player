use super::backdrop::{paint_sunset, Backdrop};
use super::format_time;
use super::textures::TextureCache;
use super::widgets::card::card;
use super::widgets::chip::chip;
use super::widgets::cover::{cover, skeleton};
use super::widgets::glass::{glass_panel, GlassKind};
use super::widgets::gradient::horizontal_gradient_mesh;
use super::widgets::icon_button::IconButton;
use super::widgets::pill_button::{PillButton, PillKind};
use super::widgets::slider::PillSlider;
use crate::library::Song;
use crate::theme::{self, icons, radius, space, text};
use eframe::egui::{self, RichText};

/// Pantalla de desarrollo (`--gallery`) con todos los widgets y estados del
/// sistema de diseño, para revisarlos sin navegar la app.
pub struct Gallery {
    backdrop: Backdrop,
    covers: Vec<String>,
    selected: Option<usize>,
    progress: f32,
    volume: f32,
    shuffle: bool,
    repeat: bool,
    lyrics: bool,
    tab: usize,
    initial_scroll: Option<f32>,
}

impl Gallery {
    pub fn new(songs: &[Song]) -> Self {
        let mut covers: Vec<String> = Vec::new();
        for path in songs.iter().filter_map(|s| s.cover_art.clone()) {
            if !covers.contains(&path) {
                covers.push(path);
            }
            if covers.len() == 8 {
                break;
            }
        }
        Self {
            backdrop: Backdrop::new(),
            selected: if covers.is_empty() { None } else { Some(0) },
            covers,
            progress: 0.35,
            volume: 0.6,
            shuffle: true,
            repeat: false,
            lyrics: false,
            tab: 0,
            initial_scroll: arg_value("--scroll").and_then(|v| v.parse().ok()),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, textures: &mut TextureCache) {
        let rect = ctx.screen_rect();
        let cover_path = self.selected.and_then(|i| self.covers.get(i)).cloned();
        self.backdrop.show(ctx, textures, cover_path.as_deref(), rect);

        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            let mut scroll = egui::ScrollArea::vertical().auto_shrink([false, false]);
            if let Some(offset) = self.initial_scroll.take() {
                scroll = scroll.vertical_scroll_offset(offset);
            }
            scroll.show(ui, |ui| {
                let margin = ((ui.available_width() - 1100.0) / 2.0).max(space::XL);
                egui::Frame::none().inner_margin(egui::Margin::symmetric(margin, space::XL)).show(ui, |ui| {
                    self.content(ui, textures);
                });
            });
        });
    }

    fn content(&mut self, ui: &mut egui::Ui, textures: &mut TextureCache) {
        let accent = self.backdrop.accent();
        ui.label(RichText::new("SIMPLE PLAYER").font(theme::deco(text::HERO)).color(accent));
        ui.label(RichText::new("Galería del sistema de diseño").font(theme::bold(text::MD)).color(theme::TEXT_MUTED));
        ui.add_space(space::LG);

        section(ui, "Fondo vivo y acento dinámico", accent, |ui| {
            ui.label(
                RichText::new("Elige una carátula: el fondo cambia con un crossfade de 400 ms y el acento sale de su color dominante.")
                    .color(theme::TEXT_MUTED),
            );
            ui.add_space(space::SM);
            ui.horizontal(|ui| {
                for i in 0..self.covers.len() {
                    let path = Some(self.covers[i].clone());
                    let response = cover(ui, textures, &path, 64.0, radius::SM, "", false);
                    let hit = ui.interact(response.rect, ui.id().with(("pick_cover", i)), egui::Sense::click());
                    if self.selected == Some(i) {
                        ui.painter().rect_stroke(response.rect.expand(3.0), radius::SM + 3.0, egui::Stroke::new(2.0_f32, accent));
                    }
                    if hit.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        self.selected = Some(i);
                    }
                }
                if chip(ui, "Sin carátula", self.selected.is_none()).clicked() {
                    self.selected = None;
                }
            });
            ui.add_space(space::MD);
            ui.horizontal(|ui| {
                let (swatch, _) = ui.allocate_exact_size(egui::vec2(120.0, 36.0), egui::Sense::hover());
                ui.painter().rect_filled(swatch, radius::SM, accent);
                ui.label(RichText::new(format!("Acento actual  #{:02X}{:02X}{:02X}", accent.r(), accent.g(), accent.b())).color(theme::TEXT_MUTED));
                let (strip, _) = ui.allocate_exact_size(egui::vec2(260.0, 10.0), egui::Sense::hover());
                ui.painter().add(horizontal_gradient_mesh(strip, &theme::GRADIENT_SUNSET));
                ui.label(RichText::new("degradado atardecer").color(theme::TEXT_MUTED).size(text::SM));
            });
        });

        section(ui, "Paleta", accent, |ui| {
            let colors = [
                ("bg_base", theme::BG_BASE),
                ("bg_dark", theme::BG_DARK),
                ("bg_sidebar", theme::BG_SIDEBAR),
                ("bg_card", theme::BG_CARD),
                ("accent_pink", theme::ACCENT_PINK),
                ("accent_lime", theme::ACCENT_LIME),
                ("accent_purple", theme::ACCENT_PURPLE),
                ("accent_sunset", theme::ACCENT_SUNSET),
                ("text_muted", theme::TEXT_MUTED),
                ("text_main", theme::TEXT_MAIN),
            ];
            ui.horizontal_wrapped(|ui| {
                for (name, color) in colors {
                    ui.vertical(|ui| {
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(92.0, 52.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, radius::SM, color);
                        ui.painter().rect_stroke(rect, radius::SM, egui::Stroke::new(1.0_f32, theme::GLASS_BORDER));
                        ui.label(RichText::new(name).size(text::XS).color(theme::TEXT_MAIN));
                        ui.label(RichText::new(format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())).size(text::XS).color(theme::TEXT_MUTED));
                    });
                }
            });
        });

        section(ui, "Tipografía", accent, |ui| {
            ui.label(RichText::new("REPRODUCIENDO AHORA").font(theme::deco(text::XL)).color(accent));
            ui.label(RichText::new("Toda la música · 628 canciones").font(theme::bold(text::LG)));
            ui.label(RichText::new("Spit Out the Bone — Metallica").font(theme::bold(text::MD)));
            ui.label(RichText::new("Cuerpo 13 px: lista de canciones, metadatos y letras con acentos áéíóú ñ ¿?").size(text::BASE));
            ui.label(RichText::new("Secundario 12 px en gris suave").size(text::SM).color(theme::TEXT_MUTED));
            ui.label(RichText::new("PIE 11 px").size(text::XS).color(theme::TEXT_MUTED));
            ui.label(RichText::new(format!("{}  {}  {}   íconos vectoriales en línea con el texto", icons::MUSIC_NOTES, icons::DISC, icons::HEART)).size(text::LG));
        });

        section(ui, "Íconos (Phosphor)", accent, |ui| {
            let set = [
                ("play", icons::PLAY), ("pause", icons::PAUSE), ("anterior", icons::SKIP_BACK), ("siguiente", icons::SKIP_FORWARD),
                ("aleatorio", icons::SHUFFLE), ("repetir", icons::REPEAT), ("repetir uno", icons::REPEAT_ONCE),
                ("volumen alto", icons::SPEAKER_HIGH), ("volumen bajo", icons::SPEAKER_LOW), ("silencio", icons::SPEAKER_X),
                ("pantalla completa", icons::ARROWS_OUT_SIMPLE), ("buscar", icons::MAGNIFYING_GLASS), ("canciones", icons::MUSIC_NOTES),
                ("álbumes", icons::DISC), ("artistas", icons::USER), ("cola", icons::QUEUE), ("letras", icons::TEXT_ALIGN_LEFT),
                ("karaoke", icons::MICROPHONE_STAGE), ("carpeta", icons::FOLDER_OPEN), ("lista", icons::LIST_BULLETS),
                ("ajustes", icons::GEAR), ("atrás", icons::CARET_LEFT), ("adelante", icons::CARET_RIGHT), ("cerrar", icons::X),
            ];
            ui.horizontal_wrapped(|ui| {
                for (name, glyph) in set {
                    IconButton::new(glyph, 44.0).accent(accent).tooltip(name).show(ui);
                }
            });
        });

        section(ui, "Botones, transporte y chips", accent, |ui| {
            ui.horizontal(|ui| {
                if IconButton::new(icons::SHUFFLE, 40.0).active(self.shuffle).accent(accent).tooltip("Aleatorio").show(ui).clicked() {
                    self.shuffle = !self.shuffle;
                }
                IconButton::new(icons::SKIP_BACK, 44.0).accent(accent).show(ui);
                IconButton::new(icons::PLAY, 60.0).primary(true).accent(accent).show(ui);
                IconButton::new(icons::SKIP_FORWARD, 44.0).accent(accent).show(ui);
                if IconButton::new(icons::REPEAT, 40.0).active(self.repeat).accent(accent).tooltip("Repetir").show(ui).clicked() {
                    self.repeat = !self.repeat;
                }
                ui.add_space(space::XL);
                if IconButton::new(icons::TEXT_ALIGN_LEFT, 40.0).active(self.lyrics).accent(accent).tooltip("Letras").show(ui).clicked() {
                    self.lyrics = !self.lyrics;
                }
            });
            ui.add_space(space::MD);
            ui.horizontal(|ui| {
                PillButton::new("Reproducir", PillKind::Primary).icon(icons::PLAY).show(ui);
                PillButton::new("Aleatorio", PillKind::Secondary).icon(icons::SHUFFLE).show(ui);
                PillButton::new("Cambiar carpeta", PillKind::Ghost).icon(icons::FOLDER_OPEN).show(ui);
            });
            ui.add_space(space::MD);
            ui.horizontal(|ui| {
                for (i, label) in ["Canciones", "Álbumes", "Artistas"].into_iter().enumerate() {
                    if chip(ui, label, self.tab == i).clicked() {
                        self.tab = i;
                    }
                }
            });
        });

        section(ui, "Sliders (pasa el mouse y arrastra)", accent, |ui| {
            let duration = 429.0;
            ui.horizontal(|ui| {
                ui.label(RichText::new(format_time((self.progress * duration) as u64)).size(text::SM).color(theme::TEXT_MUTED));
                PillSlider::new(&mut self.progress, 520.0)
                    .tooltip(move |v| format_time((v * duration) as u64))
                    .show(ui);
                ui.label(RichText::new(format_time(duration as u64)).size(text::SM).color(theme::TEXT_MUTED));
                ui.add_space(space::XL);
                ui.label(RichText::new(icons::SPEAKER_HIGH).size(text::LG).color(theme::TEXT_MUTED));
                PillSlider::new(&mut self.volume, 140.0).tooltip(|v| format!("{:.0} %", v * 100.0)).show(ui);
            });
        });

        section(ui, "Tarjetas y carátulas", accent, |ui| {
            ui.horizontal_wrapped(|ui| {
                for i in 0..4 {
                    let path = self.covers.get(i).cloned();
                    card(ui, egui::vec2(176.0, 236.0), |ui| {
                        cover(ui, textures, &path, 144.0, radius::MD, icons::MUSIC_NOTES, true);
                        ui.add_space(space::SM);
                        ui.label(RichText::new(format!("Álbum {}", i + 1)).font(theme::bold(text::BASE)));
                        ui.label(RichText::new("Artista").size(text::SM).color(theme::TEXT_MUTED));
                    });
                }
                ui.vertical(|ui| {
                    ui.label(RichText::new("Cargando").size(text::XS).color(theme::TEXT_MUTED));
                    skeleton(ui, 96.0, radius::MD);
                });
                ui.vertical(|ui| {
                    ui.label(RichText::new("Sin portada").size(text::XS).color(theme::TEXT_MUTED));
                    cover(ui, textures, &None, 96.0, radius::MD, icons::MUSIC_NOTES, false);
                });
            });
        });

        section(ui, "Paneles de vidrio", accent, |ui| {
            ui.horizontal(|ui| {
                for (kind, name) in [(GlassKind::Standard, "Estándar (blanco 7 %)"), (GlassKind::Strong, "Fuerte (índigo 55 %)")] {
                    glass_panel(ui, kind, radius::LG, space::LG, |ui| {
                        ui.set_min_size(egui::vec2(260.0, 70.0));
                        ui.label(RichText::new(name).font(theme::bold(text::BASE)));
                        ui.label(RichText::new("Relleno translúcido, borde, reflejo superior y sombra suave.").size(text::SM).color(theme::TEXT_MUTED));
                    });
                }
            });
        });

        let _ = paint_sunset;
    }
}

fn section(ui: &mut egui::Ui, title: &str, accent: egui::Color32, add_contents: impl FnOnce(&mut egui::Ui)) {
    glass_panel(ui, GlassKind::Standard, radius::LG, space::XL, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(title.to_uppercase()).font(theme::deco(text::LG)).color(accent));
        ui.add_space(space::MD);
        add_contents(ui);
    });
    ui.add_space(space::LG);
}

/// Valor de un flag de desarrollo, p. ej. `--scroll 800`.
pub fn arg_value(flag: &str) -> Option<String> {
    let mut args = std::env::args();
    args.find(|a| a == flag).and_then(|_| args.next())
}
