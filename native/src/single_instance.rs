use std::io;
use std::path::{Path, PathBuf};

pub enum Acquire {
    Acquired(InstanceLock),
    AlreadyRunning,
}

/// Mantiene viva la instancia única: mientras exista, otra copia de la app
/// detecta que ya hay una abierta (dos instancias pelean los atajos globales
/// y X mata el proceso con BadAccess).
pub struct InstanceLock {
    #[cfg(unix)]
    _listener: std::os::unix::net::UnixListener,
    path: PathBuf,
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub fn default_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    dir.join("simple-player.sock")
}

#[cfg(unix)]
pub fn acquire(path: &Path) -> io::Result<Acquire> {
    use std::os::unix::net::{UnixListener, UnixStream};

    let bind = |path: &Path| UnixListener::bind(path).map(|l| Acquire::Acquired(InstanceLock { _listener: l, path: path.to_path_buf() }));
    match bind(path) {
        Err(e) if e.kind() == io::ErrorKind::AddrInUse => match UnixStream::connect(path) {
            // Alguien escucha: hay otra instancia viva (o su cola está llena).
            Ok(_) => Ok(Acquire::AlreadyRunning),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(Acquire::AlreadyRunning),
            // Nadie escucha: el socket quedó de un cierre brusco.
            Err(_) => {
                std::fs::remove_file(path)?;
                bind(path)
            }
        },
        other => other,
    }
}

#[cfg(not(unix))]
pub fn acquire(path: &Path) -> io::Result<Acquire> {
    Ok(Acquire::Acquired(InstanceLock { path: path.to_path_buf() }))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    fn temp_socket(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("sp_test_{}_{name}.sock", std::process::id()))
    }

    #[test]
    fn la_primera_instancia_se_adquiere_y_la_segunda_ve_que_ya_hay_una() {
        let path = temp_socket("dos");
        let first = acquire(&path).unwrap();
        assert!(matches!(first, Acquire::Acquired(_)));
        assert!(matches!(acquire(&path).unwrap(), Acquire::AlreadyRunning));
    }

    #[test]
    fn al_cerrar_la_primera_se_puede_volver_a_abrir() {
        let path = temp_socket("reabrir");
        drop(acquire(&path).unwrap());
        assert!(matches!(acquire(&path).unwrap(), Acquire::Acquired(_)));
    }

    #[test]
    fn un_socket_huerfano_de_un_cierre_brusco_no_bloquea_el_arranque() {
        let path = temp_socket("huerfano");
        drop(UnixListener::bind(&path).unwrap());
        assert!(path.exists(), "el archivo queda aunque el proceso murió");
        assert!(matches!(acquire(&path).unwrap(), Acquire::Acquired(_)));
    }
}
