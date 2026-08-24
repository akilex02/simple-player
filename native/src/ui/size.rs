/// Equivalente al prop `size: "compact" | "large"` de TransportButtons/VolumeControl
/// en la versión React — mismos controles, reutilizados entre la barra de
/// reproducción y la pantalla completa con distinto tamaño.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Size {
    Compact,
    Large,
}
