import type { RefObject } from "react";
import { GlowPulse } from "./visualizers/GlowPulse";
import { RadialSpectrum } from "./visualizers/RadialSpectrum";
import { SpectrumBars } from "./visualizers/SpectrumBars";

export type VisualizerMode = "bars" | "radial" | "glow";

export const VISUALIZER_MODES: VisualizerMode[] = ["bars", "radial", "glow"];

export const VISUALIZER_LABELS: Record<VisualizerMode, string> = {
  bars: "Barras",
  radial: "Radial",
  glow: "Resplandor",
};

interface VisualizerBackgroundProps {
  mode: VisualizerMode;
  dataRef: RefObject<number[]>;
}

export function VisualizerBackground({ mode, dataRef }: VisualizerBackgroundProps) {
  return (
    <div className="fullscreen-bg">
      {mode === "bars" && <SpectrumBars dataRef={dataRef} />}
      {mode === "radial" && <RadialSpectrum dataRef={dataRef} />}
      {mode === "glow" && <GlowPulse dataRef={dataRef} />}
    </div>
  );
}
