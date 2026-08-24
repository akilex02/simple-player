import { useEffect, useRef, type RefObject } from "react";
import { SPECTRUM_MAX_DB, SPECTRUM_MIN_DB } from "../../hooks/useSpectrumData";

interface SpectrumBarsProps {
  dataRef: RefObject<number[]>;
}

export function SpectrumBars({ dataRef }: SpectrumBarsProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    const ctx = canvas?.getContext("2d");
    if (!canvas || !ctx) return;

    const resize = () => {
      canvas.width = canvas.clientWidth;
      canvas.height = canvas.clientHeight;
    };
    resize();
    window.addEventListener("resize", resize);

    let rafId = 0;
    const draw = () => {
      rafId = requestAnimationFrame(draw);
      const data = dataRef.current;
      const { width, height } = canvas;
      ctx.clearRect(0, 0, width, height);

      const gap = 4;
      const barWidth = (width - gap * (data.length - 1)) / data.length;
      const gradient = ctx.createLinearGradient(0, height, 0, 0);
      gradient.addColorStop(0, "#ff9ebd");
      gradient.addColorStop(1, "#fef8c9");
      ctx.fillStyle = gradient;

      for (let i = 0; i < data.length; i++) {
        const norm = Math.max(0, Math.min(1, (data[i] - SPECTRUM_MIN_DB) / (SPECTRUM_MAX_DB - SPECTRUM_MIN_DB)));
        const barHeight = norm * height;
        ctx.fillRect(i * (barWidth + gap), height - barHeight, barWidth, barHeight);
      }
    };
    draw();

    return () => {
      cancelAnimationFrame(rafId);
      window.removeEventListener("resize", resize);
    };
  }, [dataRef]);

  return <canvas ref={canvasRef} className="visualizer-canvas" />;
}
