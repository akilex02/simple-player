//! Utilización de la GPU para `--bench` (solo desarrollo). Sin dependencias: lee
//! `nvidia-smi` (NVIDIA) o `gpu_busy_percent` de sysfs (AMD) y, si no hay ninguno, informa `n/d`.
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Un porcentaje válido (0..=100) a partir de texto como `37`, ` 37 ` o `37 %`.
fn parse_percent(text: &str) -> Option<f32> {
    let value: f32 = text.trim().trim_end_matches('%').trim().parse().ok()?;
    (0.0..=100.0).contains(&value).then_some(value)
}

pub fn parse_nvidia_line(line: &str) -> Option<f32> {
    parse_percent(line)
}

pub fn parse_sysfs_busy(text: &str) -> Option<f32> {
    parse_percent(text)
}

pub fn mean(samples: &[f32]) -> Option<f32> {
    (!samples.is_empty()).then(|| samples.iter().sum::<f32>() / samples.len() as f32)
}

/// Archivo `gpu_busy_percent` de la primera GPU AMD que lo expone.
fn find_sysfs_busy_file() -> Option<PathBuf> {
    std::fs::read_dir("/sys/class/drm").ok()?.flatten().map(|e| e.path().join("device/gpu_busy_percent")).find(|p| p.exists())
}

pub struct GpuProbe {
    samples: Arc<Mutex<Vec<f32>>>,
    source: &'static str,
    child: Option<Child>,
}

impl GpuProbe {
    /// Sin fuente: el reporte siempre es `GPU n/d`.
    pub fn disabled() -> Self {
        Self { samples: Arc::new(Mutex::new(Vec::new())), source: "", child: None }
    }

    /// Intenta `nvidia-smi` (una muestra por segundo) y luego sysfs.
    pub fn start() -> Self {
        let samples = Arc::new(Mutex::new(Vec::new()));
        let spawned = Command::new("nvidia-smi")
            .args(["--query-gpu=utilization.gpu", "--format=csv,noheader,nounits", "-l", "1"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        if let Ok(mut child) = spawned {
            if let Some(stdout) = child.stdout.take() {
                let sink = Arc::clone(&samples);
                std::thread::spawn(move || {
                    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                        if let Some(value) = parse_nvidia_line(&line) {
                            sink.lock().unwrap().push(value);
                        }
                    }
                });
                return Self { samples, source: "nvidia-smi", child: Some(child) };
            }
        }
        if let Some(path) = find_sysfs_busy_file() {
            let sink = Arc::clone(&samples);
            std::thread::spawn(move || loop {
                if let Some(value) = std::fs::read_to_string(&path).ok().and_then(|t| parse_sysfs_busy(&t)) {
                    sink.lock().unwrap().push(value);
                }
                std::thread::sleep(Duration::from_secs(1));
            });
            return Self { samples, source: "sysfs", child: None };
        }
        Self::disabled()
    }

    /// Descarta lo muestreado hasta ahora (el calentamiento no cuenta).
    pub fn begin_window(&self) {
        self.samples.lock().unwrap().clear();
    }

    pub fn report(&self) -> String {
        let samples = self.samples.lock().unwrap();
        match mean(&samples) {
            Some(avg) => format!("GPU {avg:.0} % ({}, {} muestras)", self.source, samples.len()),
            None => "GPU n/d".to_string(),
        }
    }
}

impl Drop for GpuProbe {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_la_linea_de_nvidia_smi() {
        assert_eq!(parse_nvidia_line("37"), Some(37.0));
        assert_eq!(parse_nvidia_line(" 37 \n"), Some(37.0));
        assert_eq!(parse_nvidia_line("37 %"), Some(37.0));
        assert_eq!(parse_nvidia_line("[N/A]"), None);
        assert_eq!(parse_nvidia_line(""), None);
    }

    #[test]
    fn lee_el_valor_de_sysfs() {
        assert_eq!(parse_sysfs_busy("42\n"), Some(42.0));
        assert_eq!(parse_sysfs_busy("abc"), None);
    }

    #[test]
    fn rechaza_porcentajes_imposibles() {
        assert_eq!(parse_nvidia_line("250"), None);
        assert_eq!(parse_sysfs_busy("-3"), None);
    }

    #[test]
    fn la_media_de_nada_es_nada() {
        assert_eq!(mean(&[]), None);
        assert_eq!(mean(&[10.0, 20.0, 30.0]), Some(20.0));
    }

    #[test]
    fn sin_muestras_el_reporte_dice_no_disponible() {
        let probe = GpuProbe::disabled();
        assert_eq!(probe.report(), "GPU n/d");
    }
}
