use eframe::egui;

// Tokens de color — misma paleta que `:root` en App.css (versión Tauri).
pub const BG_DARK: egui::Color32 = egui::Color32::from_rgb(0x13, 0x13, 0x22);
pub const BG_SIDEBAR: egui::Color32 = egui::Color32::from_rgb(0x18, 0x18, 0x2d);
pub const BG_CARD: egui::Color32 = egui::Color32::from_rgb(0x22, 0x22, 0x38);
pub const BG_CARD_HOVER: egui::Color32 = egui::Color32::from_rgb(0x2b, 0x2b, 0x48);

pub const ACCENT_PINK: egui::Color32 = egui::Color32::from_rgb(0xff, 0x9e, 0xbd);
pub const ACCENT_LIME: egui::Color32 = egui::Color32::from_rgb(0xfe, 0xf8, 0xc9);
pub const ACCENT_PINK_HOVER: egui::Color32 = egui::Color32::from_rgb(0xff, 0x7b, 0xa4);

pub const TEXT_MAIN: egui::Color32 = egui::Color32::from_rgb(0xff, 0xff, 0xff);
pub const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(0xa4, 0xa4, 0xc4);
pub const BORDER_SUBTLE: egui::Color32 = egui::Color32::from_rgb(0x28, 0x28, 0x42);

pub const FONT_CONDENSED: &str = "GTAArtDecoCondensed";
pub const FONT_REGULAR: &str = "GTAArtDeco";

/// Carga las fuentes GTAArtDeco (convertidas de WOFF a TTF) y aplica la
/// paleta de colores base al contexto de egui.
pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let font_files: &[(&str, &[u8])] = &[
        (FONT_REGULAR, include_bytes!("../assets/fonts/GTAArtDeco_Regular.ttf")),
        (FONT_CONDENSED, include_bytes!("../assets/fonts/GTAArtDeco_CondensedBold.ttf")),
    ];

    for (name, bytes) in font_files {
        fonts
            .font_data
            .insert(name.to_string(), egui::FontData::from_static(bytes));
        fonts
            .families
            .entry(egui::FontFamily::Name((*name).into()))
            .or_default()
            .insert(0, name.to_string());
    }

    // La fuente "Proportional" por defecto pasa a ser GTAArtDeco Regular.
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, FONT_REGULAR.to_string());

    ctx.set_fonts(fonts);

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG_DARK;
    visuals.window_fill = BG_DARK;
    visuals.override_text_color = Some(TEXT_MAIN);
    ctx.set_visuals(visuals);
}
