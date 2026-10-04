//! Registro del AppImage en el escritorio (como hace AppImageLauncher): un `.desktop` y el ícono en el
//! directorio de usuario. Sin esto, un AppImage suelto no tiene ícono en la barra de tareas ni controles
//! multimedia en Plasma, porque el escritorio relaciona la ventana con su `.desktop`.
use std::path::{Path, PathBuf};

pub const ICON_NAME: &str = "simple-player";

/// Dónde se escriben los dos archivos (en el directorio de datos del usuario).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paths {
    pub desktop: PathBuf,
    pub icon: PathBuf,
}

pub fn paths(xdg_data_home: Option<&str>, home: Option<&str>) -> Paths {
    let base = match xdg_data_home.filter(|v| !v.is_empty()) {
        Some(xdg) => PathBuf::from(xdg),
        None => PathBuf::from(home.unwrap_or("/tmp")).join(".local").join("share"),
    };
    Paths {
        desktop: base.join("applications").join(format!("{ICON_NAME}.desktop")),
        icon: base.join("icons/hicolor/512x512/apps").join(format!("{ICON_NAME}.png")),
    }
}

/// Ruta del AppImage en ejecución (`APPIMAGE`), o `None` si la app no corre como AppImage.
pub fn appimage_path() -> Option<PathBuf> {
    std::env::var_os("APPIMAGE").filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// Entrecomilla un argumento de `Exec=` según la especificación de Desktop Entry: comillas dobles, y con
/// barra invertida delante de `"`, `` ` ``, `$` y `\`; como el valor del archivo vuelve a procesar las
/// barras, cada una se duplica; el `%` se escribe `%%`.
pub fn exec_quote(arg: &str) -> String {
    let mut out = String::from("\"");
    for c in arg.chars() {
        match c {
            '\\' => out.push_str("\\\\\\\\"),
            '"' | '`' | '$' => {
                out.push('\\');
                out.push(c);
            }
            '%' => out.push_str("%%"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn exec_line(appimage: &Path) -> String {
    format!("Exec={} %U", exec_quote(&appimage.to_string_lossy()))
}

/// Contenido del `.desktop` que lanza ese AppImage.
pub fn desktop_entry(appimage: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Simple Player\nComment=Reproductor de música local\n{}\nIcon={ICON_NAME}\nCategories=Audio;Music;Player;AudioVideo;\nStartupWMClass=simple-player\n",
        exec_line(appimage)
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    NotInstalled,
    Installed,
    /// Hay un `.desktop` pero apunta a otro lugar (el AppImage se movió) o no es legible.
    Outdated,
}

pub fn status(paths: &Paths, appimage: &Path) -> Status {
    let Ok(text) = std::fs::read_to_string(&paths.desktop) else {
        return Status::NotInstalled;
    };
    if text.lines().any(|line| line == exec_line(appimage)) {
        Status::Installed
    } else {
        Status::Outdated
    }
}

pub fn install(paths: &Paths, appimage: &Path, icon_png: &[u8]) -> std::io::Result<()> {
    for file in [&paths.desktop, &paths.icon] {
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
    }
    std::fs::write(&paths.icon, icon_png)?;
    std::fs::write(&paths.desktop, desktop_entry(appimage))
}

/// Borra los dos archivos; lo que ya no existe no es un error.
pub fn remove(paths: &Paths) -> std::io::Result<()> {
    for file in [&paths.desktop, &paths.icon] {
        match std::fs::remove_file(file) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Comandos que refrescan las cachés del escritorio tras tocar el `.desktop` o el ícono.
fn cache_commands(paths: &Paths) -> Vec<(&'static str, Vec<String>)> {
    let mut cmds = Vec::new();
    if let Some(dir) = paths.desktop.parent() {
        cmds.push(("update-desktop-database", vec![dir.to_string_lossy().into_owned()]));
    }
    // .../icons/hicolor/512x512/apps/simple-player.png -> .../icons/hicolor
    if let Some(theme) = paths.icon.ancestors().nth(3) {
        cmds.push(("gtk-update-icon-cache", vec!["-f".into(), "-t".into(), theme.to_string_lossy().into_owned()]));
    }
    cmds.push(("kbuildsycoca6", vec![]));
    cmds
}

/// Ejecuta cada comando en un hilo, uno tras otro, sin esperar y sin quejarse si no existe o falla.
fn run_best_effort(cmds: Vec<(&'static str, Vec<String>)>) {
    std::thread::spawn(move || {
        for (prog, args) in cmds {
            let _ = std::process::Command::new(prog)
                .args(args)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }
    });
}

/// Pide al escritorio que relea el `.desktop` y el ícono (mejor esfuerzo; Plasma las cachea).
pub fn refresh_caches(paths: &Paths) {
    run_best_effort(cache_commands(paths));
}

/// ¿Se ofrece registrar la app? Solo como AppImage, si aún no está registrada (o apunta a otro sitio) y
/// el usuario no lo descartó (para siempre o en esta sesión).
pub fn should_prompt(is_appimage: bool, status: Status, dismissed_forever: bool, dismissed_session: bool) -> bool {
    is_appimage && status != Status::Installed && !dismissed_forever && !dismissed_session
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sp_integ_{}_{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn los_comandos_de_cache_apuntan_a_las_carpetas_del_usuario() {
        let p = paths(Some("/datos"), Some("/home/u"));
        let cmds = cache_commands(&p);
        assert!(cmds.contains(&("update-desktop-database", vec!["/datos/applications".to_string()])));
        assert!(cmds.contains(&("gtk-update-icon-cache", vec!["-f".into(), "-t".into(), "/datos/icons/hicolor".into()])));
        assert!(cmds.iter().any(|(prog, _)| *prog == "kbuildsycoca6"));
    }

    #[test]
    fn un_comando_inexistente_se_omite_sin_error() {
        run_best_effort(vec![("sp-comando-que-no-existe-xyz", vec![])]);
    }

    #[test]
    fn las_rutas_respetan_xdg_data_home() {
        let p = paths(Some("/datos"), Some("/home/u"));
        assert_eq!(p.desktop, PathBuf::from("/datos/applications/simple-player.desktop"));
        assert_eq!(p.icon, PathBuf::from("/datos/icons/hicolor/512x512/apps/simple-player.png"));
        let p = paths(None, Some("/home/u"));
        assert_eq!(p.desktop, PathBuf::from("/home/u/.local/share/applications/simple-player.desktop"));
        assert_eq!(paths(Some(""), Some("/home/u")).desktop, p.desktop, "XDG vacío = por defecto");
    }

    #[test]
    fn el_exec_se_entrecomilla_segun_la_especificacion() {
        assert_eq!(exec_quote("/opt/App.AppImage"), r#""/opt/App.AppImage""#);
        assert_eq!(exec_quote("/mi carpeta/App.AppImage"), r#""/mi carpeta/App.AppImage""#, "espacios");
        assert_eq!(exec_quote(r#"/a"b"#), r#""/a\"b""#, "comillas");
        assert_eq!(exec_quote("/a$b"), r#""/a\$b""#, "dólar");
        assert_eq!(exec_quote("/a`b"), r#""/a\`b""#, "acento grave");
        assert_eq!(exec_quote(r"/a\b"), r#""/a\\\\b""#, "barra invertida: se duplica dos veces");
        assert_eq!(exec_quote("/a%b"), r#""/a%%b""#, "porcentaje");
    }

    #[test]
    fn el_desktop_apunta_al_appimage_y_trae_el_nombre_el_icono_y_la_clase() {
        let text = desktop_entry(Path::new("/home/u/Descargas/Simple Player.AppImage"));
        assert!(text.starts_with("[Desktop Entry]\n"));
        assert!(text.contains("Name=Simple Player\n"));
        assert!(text.contains("Icon=simple-player\n"));
        assert!(text.contains("StartupWMClass=simple-player\n"));
        assert!(text.contains(r#"Exec="/home/u/Descargas/Simple Player.AppImage" %U"#), "{text}");
        assert!(text.contains("Type=Application\n"));
    }

    #[test]
    fn instalar_escribe_el_desktop_y_el_icono_y_quitar_los_borra() {
        let dir = temp_dir("install");
        let p = paths(Some(dir.to_str().unwrap()), None);
        let app = Path::new("/opt/Simple.AppImage");
        assert_eq!(status(&p, app), Status::NotInstalled);

        install(&p, app, b"PNGDATA").unwrap();
        assert_eq!(status(&p, app), Status::Installed);
        assert_eq!(fs::read(&p.icon).unwrap(), b"PNGDATA");
        assert!(fs::read_to_string(&p.desktop).unwrap().contains("/opt/Simple.AppImage"));

        remove(&p).unwrap();
        assert_eq!(status(&p, app), Status::NotInstalled);
        assert!(!p.desktop.exists() && !p.icon.exists());
        remove(&p).unwrap(); // quitar de nuevo no es un error
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn si_el_appimage_cambio_de_sitio_el_estado_es_desactualizado() {
        let dir = temp_dir("moved");
        let p = paths(Some(dir.to_str().unwrap()), None);
        install(&p, Path::new("/viejo/Simple.AppImage"), b"x").unwrap();
        assert_eq!(status(&p, Path::new("/nuevo/Simple.AppImage")), Status::Outdated);
        install(&p, Path::new("/nuevo/Simple.AppImage"), b"x").unwrap();
        assert_eq!(status(&p, Path::new("/nuevo/Simple.AppImage")), Status::Installed);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn un_desktop_sin_exec_o_ilegible_cuenta_como_desactualizado_no_como_error() {
        let dir = temp_dir("broken");
        let p = paths(Some(dir.to_str().unwrap()), None);
        fs::create_dir_all(p.desktop.parent().unwrap()).unwrap();
        fs::write(&p.desktop, "[Desktop Entry]\nName=Otra cosa\n").unwrap();
        assert_eq!(status(&p, Path::new("/opt/Simple.AppImage")), Status::Outdated);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn el_aviso_solo_sale_como_appimage_sin_registrar_y_sin_haberlo_descartado() {
        // (es_appimage, estado, descartado_para_siempre, descartado_en_esta_sesion)
        assert!(should_prompt(true, Status::NotInstalled, false, false));
        assert!(should_prompt(true, Status::Outdated, false, false), "ruta vieja: se vuelve a ofrecer");
        assert!(!should_prompt(true, Status::Installed, false, false), "ya registrado");
        assert!(!should_prompt(false, Status::NotInstalled, false, false), "no es AppImage");
        assert!(!should_prompt(true, Status::NotInstalled, true, false), "no volver a preguntar");
        assert!(!should_prompt(true, Status::NotInstalled, false, true), "ahora no");
    }
}
