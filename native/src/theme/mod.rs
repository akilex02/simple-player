pub mod blur;
pub mod color;
pub mod icons;

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle};

// ── Paleta ────────────────────────────────────────────────────────────────
pub const BG_BASE: Color32 = Color32::from_rgb(0x0e, 0x0e, 0x1b);
pub const BG_DARK: Color32 = Color32::from_rgb(0x13, 0x13, 0x22);
pub const BG_SIDEBAR: Color32 = Color32::from_rgb(0x18, 0x18, 0x2d);
pub const BG_CARD: Color32 = Color32::from_rgb(0x22, 0x22, 0x38);
pub const BG_CARD_HOVER: Color32 = Color32::from_rgb(0x2b, 0x2b, 0x48);

pub const ACCENT_PINK: Color32 = Color32::from_rgb(0xff, 0x9e, 0xbd);
pub const ACCENT_PINK_HOVER: Color32 = Color32::from_rgb(0xff, 0x7b, 0xa4);
pub const ACCENT_LIME: Color32 = Color32::from_rgb(0xfe, 0xf8, 0xc9);
pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(0x9d, 0x8e, 0xc4);
pub const ACCENT_SUNSET: Color32 = Color32::from_rgb(0xff, 0xb3, 0x6b);

pub const TEXT_MAIN: Color32 = Color32::from_rgb(0xff, 0xff, 0xff);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0xa4, 0xa4, 0xc4);
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(0x28, 0x28, 0x42);

/// Degradado "atardecer": rosa → naranja → púrpura.
pub const GRADIENT_SUNSET: [(f32, Color32); 3] = [(0.0, ACCENT_PINK), (0.5, ACCENT_SUNSET), (1.0, ACCENT_PURPLE)];

// ── Vidrio (valores premultiplicados para poder ser `const`) ───────────────
/// Blanco al 7 %.
pub const GLASS_FILL: Color32 = Color32::from_rgba_premultiplied(18, 18, 18, 18);
/// `#131322` al 55 %: sidebar y barra inferior.
pub const GLASS_FILL_STRONG: Color32 = Color32::from_rgba_premultiplied(10, 10, 19, 140);
/// Blanco al 8 %.
pub const GLASS_BORDER: Color32 = Color32::from_rgba_premultiplied(20, 20, 20, 20);
/// Reflejo de la línea superior: blanco al 18 %.
pub const GLASS_HIGHLIGHT: Color32 = Color32::from_rgba_premultiplied(46, 46, 46, 46);

pub mod radius {
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
    pub const PILL: f32 = 9999.0;
}

pub mod space {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
    pub const XXL: f32 = 32.0;
}

/// Escala tipográfica.
pub mod text {
    pub const XS: f32 = 11.0;
    pub const SM: f32 = 12.0;
    pub const BASE: f32 = 13.0;
    pub const MD: f32 = 15.0;
    pub const LG: f32 = 18.0;
    pub const XL: f32 = 24.0;
    pub const XXL: f32 = 32.0;
    pub const HERO: f32 = 48.0;
}

/// Duraciones de animación en segundos.
pub mod motion {
    pub const HOVER: f32 = 0.12;
    pub const SCREEN: f32 = 0.25;
    pub const BACKDROP: f32 = 0.4;
}

// ── Fuentes ───────────────────────────────────────────────────────────────
pub const FONT_CONDENSED: &str = "GTAArtDecoCondensed";
pub const FONT_REGULAR: &str = "GTAArtDeco";
pub const FONT_BOLD: &str = "NotoSansBold";

/// Texto de interfaz en negrita (egui no tiene pesos: es otra familia).
pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(FONT_BOLD.into()))
}

/// Art Deco condensado: logo y títulos de sección.
pub fn deco(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(FONT_CONDENSED.into()))
}

/// Acento dinámico del momento (sale de la carátula actual). Lo publica el
/// fondo cada frame y lo leen los widgets que no reciben un acento explícito.
pub fn set_accent(ctx: &egui::Context, color: Color32) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("dynamic_accent"), color));
}

pub fn accent(ctx: &egui::Context) -> Color32 {
    ctx.data(|d| d.get_temp(egui::Id::new("dynamic_accent"))).unwrap_or(ACCENT_PINK)
}

pub fn with_alpha(c: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        mix(a.r(), b.r()),
        mix(a.g(), b.g()),
        mix(a.b(), b.b()),
        mix(a.a(), b.a()),
    )
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let font_files: &[(&str, &[u8])] = &[
        (FONT_REGULAR, include_bytes!("../../assets/fonts/GTAArtDeco_Regular.ttf")),
        (FONT_CONDENSED, include_bytes!("../../assets/fonts/GTAArtDeco_CondensedBold.ttf")),
        ("NotoSans", include_bytes!("../../assets/fonts/NotoSans-Regular.ttf")),
        (FONT_BOLD, include_bytes!("../../assets/fonts/NotoSans-Bold.ttf")),
    ];
    for (name, bytes) in font_files {
        fonts.font_data.insert(name.to_string(), FontData::from_static(bytes));
    }

    // Interfaz: Noto Sans primero, luego Phosphor (íconos) y las fuentes por
    // defecto de egui, que aportan cobertura de emoji.
    fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "NotoSans".into());
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    let fallback: Vec<String> = fonts.families[&FontFamily::Proportional]
        .iter()
        .filter(|name| name.as_str() != "NotoSans")
        .cloned()
        .collect();

    let named: [(&str, &str); 2] = [(FONT_BOLD, FONT_BOLD), (FONT_CONDENSED, FONT_CONDENSED)];
    for (family, primary) in named {
        let mut chain = vec![primary.to_string(), "NotoSans".to_string()];
        chain.extend(fallback.iter().cloned());
        fonts.families.insert(FontFamily::Name(family.into()), chain);
    }
    let mut regular = vec![FONT_REGULAR.to_string(), "NotoSans".to_string()];
    regular.extend(fallback);
    fonts.families.insert(FontFamily::Name(FONT_REGULAR.into()), regular);

    ctx.set_fonts(fonts);
}

fn install_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.text_styles = [
        (TextStyle::Small, FontId::proportional(text::XS)),
        (TextStyle::Body, FontId::proportional(text::BASE)),
        (TextStyle::Button, FontId::proportional(text::BASE)),
        (TextStyle::Heading, deco(text::XL)),
        (TextStyle::Monospace, FontId::monospace(text::SM)),
    ]
    .into();

    style.spacing.item_spacing = egui::vec2(space::SM, 6.0);
    style.spacing.button_padding = egui::vec2(space::MD, 6.0);
    style.spacing.scroll = egui::style::ScrollStyle::floating();

    let v = &mut style.visuals;
    v.dark_mode = true;
    v.panel_fill = BG_DARK;
    v.window_fill = BG_DARK;
    v.extreme_bg_color = BG_BASE;
    v.faint_bg_color = GLASS_FILL;
    v.override_text_color = Some(TEXT_MAIN);
    v.window_rounding = radius::LG.into();
    v.menu_rounding = radius::MD.into();
    v.window_stroke = Stroke::new(1.0_f32, GLASS_BORDER);
    v.selection.bg_fill = with_alpha(ACCENT_PINK, 90);
    v.selection.stroke = Stroke::new(1.0_f32, ACCENT_PINK);
    v.hyperlink_color = ACCENT_PINK;

    let rounding = egui::Rounding::same(radius::SM);
    v.widgets.noninteractive.rounding = rounding;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, GLASS_BORDER);
    for (state, fill, stroke) in [
        (&mut v.widgets.inactive, GLASS_FILL, GLASS_BORDER),
        (&mut v.widgets.hovered, Color32::from_white_alpha(34), with_alpha(ACCENT_PINK, 120)),
        (&mut v.widgets.active, with_alpha(ACCENT_PINK, 70), ACCENT_PINK),
        (&mut v.widgets.open, Color32::from_white_alpha(28), GLASS_BORDER),
    ] {
        state.rounding = rounding;
        state.weak_bg_fill = fill;
        state.bg_fill = fill;
        state.bg_stroke = Stroke::new(1.0_f32, stroke);
        state.expansion = 0.0;
    }

    ctx.set_style(style);
}

/// Instala fuentes y el estilo global (colores, radios, tipografía).
pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);
    install_style(ctx);
}
