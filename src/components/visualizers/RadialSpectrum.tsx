import { useEffect, useRef, type RefObject } from "react";
import { SPECTRUM_MAX_DB, SPECTRUM_MIN_DB } from "../../hooks/useSpectrumData";

interface RadialSpectrumProps {
  dataRef: RefObject<number[]>;
}

export function RadialSpectrum({ dataRef }: RadialSpectrumProps) {
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

      const cx = width / 2;
      const cy = height / 2;
      const innerRadius = Math.min(width, height) * 0.16;
      const maxBarLength = Math.min(width, height) * 0.22;
      const lineWidth = Math.max(2, ((2 * Math.PI * innerRadius) / data.length) * 0.6);

      ctx.strokeStyle = "#ff9ebd";
      ctx.lineWidth = lineWidth;
      ctx.lineCap = "round";

      for (let i = 0; i < data.length; i++) {
        const norm = Math.max(0, Math.min(1, (data[i] - SPECTRUM_MIN_DB) / (SPECTRUM_MAX_DB - SPECTRUM_MIN_DB)));
        const barLength = norm * maxBarLength;
        const angle = (i / data.length) * Math.PI * 2 - Math.PI / 2;
        const cos = Math.cos(angle);
        const sin = Math.sin(angle);

        ctx.beginPath();
        ctx.moveTo(cx + cos * innerRadius, cy + sin * innerRadius);
        ctx.lineTo(cx + cos * (innerRadius + barLength), cy + sin * (innerRadius + barLength));
        ctx.stroke();
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
