import { useEffect, useRef, type RefObject } from "react";
import { SPECTRUM_MAX_DB, SPECTRUM_MIN_DB } from "../../hooks/useSpectrumData";

interface GlowPulseProps {
  dataRef: RefObject<number[]>;
}

export function GlowPulse({ dataRef }: GlowPulseProps) {
  const glowRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let rafId = 0;
    const draw = () => {
      rafId = requestAnimationFrame(draw);
      const el = glowRef.current;
      const data = dataRef.current;
      if (!el || data.length === 0) return;

      const avgDb = data.reduce((sum, v) => sum + v, 0) / data.length;
      const norm = Math.max(0, Math.min(1, (avgDb - SPECTRUM_MIN_DB) / (SPECTRUM_MAX_DB - SPECTRUM_MIN_DB)));

      el.style.transform = `scale(${(0.9 + norm * 0.35).toFixed(3)})`;
      el.style.opacity = (0.35 + norm * 0.5).toFixed(3);
    };
    draw();
    return () => cancelAnimationFrame(rafId);
  }, [dataRef]);

  return <div ref={glowRef} className="visualizer-glow" />;
}
