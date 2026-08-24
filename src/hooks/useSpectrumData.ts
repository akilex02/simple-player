import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";

export const SPECTRUM_BANDS = 32;
export const SPECTRUM_MIN_DB = -60;
export const SPECTRUM_MAX_DB = 0;

/**
 * Expone las magnitudes del elemento `spectrum` de GStreamer (en dB, 32 bandas)
 * como un ref actualizado ~20 veces/seg, sin causar re-renders de React — los
 * visualizadores lo leen directamente dentro de su propio loop de
 * requestAnimationFrame para dibujar en canvas o mutar estilos a mano.
 */
export function useSpectrumData() {
  const dataRef = useRef<number[]>(new Array(SPECTRUM_BANDS).fill(SPECTRUM_MIN_DB));

  useEffect(() => {
    const unlisten = listen<number[]>("spectrum-data", event => {
      dataRef.current = event.payload;
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  return dataRef;
}
