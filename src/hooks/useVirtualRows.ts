import { useEffect, useRef, useState } from "react";

interface VirtualRange {
  start: number;
  end: number;
}

/**
 * Virtualización simple para listas de alto de fila fijo: solo calcula qué
 * rango de índices cae dentro del viewport visible del ancestro con scroll
 * (buscado vía `closest(scrollSelector)`), más un colchón (`overscan`) de
 * filas arriba/abajo para que no "parpadee" al hacer scroll rápido.
 *
 * El componente consumidor sigue siendo responsable de renderizar solo
 * `range.start..range.end` y de reservar el alto total (`itemCount * rowHeight`)
 * para que la barra de scroll se comporte igual que si todo estuviera montado.
 */
export function useVirtualRows(
  itemCount: number,
  rowHeight: number,
  scrollSelector: string,
  overscan = 6
) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [range, setRange] = useState<VirtualRange>({ start: 0, end: Math.min(itemCount, 30) });

  useEffect(() => {
    const rowsEl = containerRef.current;
    const scrollEl = rowsEl?.closest(scrollSelector) as HTMLElement | null;
    if (!rowsEl || !scrollEl) return;

    let rafId = 0;
    let retryTimeoutId = 0;

    const update = () => {
      try {
        const scrollRect = scrollEl.getBoundingClientRect();
        const rowsRect = rowsEl.getBoundingClientRect();
        // Posición del inicio de la lista relativa al contenido con scroll
        // (no depende de position:relative/absolute de ningún ancestro).
        const rowsOffset = rowsRect.top - scrollRect.top + scrollEl.scrollTop;
        const relativeScroll = Math.max(0, scrollEl.scrollTop - rowsOffset);

        // Piso mínimo de filas visibles: si clientHeight se lee mal (medido
        // antes de que el layout termine de estabilizarse, algo que ya vimos
        // pasar con otras APIs en este WebKitGTK), nunca renderizamos menos
        // de MIN_VISIBLE filas — sigue siendo una fracción mínima de 628.
        const MIN_VISIBLE = 24;
        const visibleCount = Math.max(
          MIN_VISIBLE,
          Math.ceil(scrollEl.clientHeight / rowHeight) + overscan * 2
        );

        let start = Math.max(0, Math.floor(relativeScroll / rowHeight) - overscan);
        // No dejar el rango "pegado" cerca del final si itemCount cambió
        // (p. ej. un filtro de búsqueda redujo la lista) mientras el scroll
        // seguía desplazado desde antes.
        start = Math.min(start, Math.max(0, itemCount - visibleCount));
        const end = Math.min(itemCount, start + visibleCount);

        setRange(prev => (prev.start === start && prev.end === end) ? prev : { start, end });
      } catch (err) {
        console.error("useVirtualRows: fallo al calcular el rango visible", err);
      }
    };

    const onScroll = () => {
      if (rafId) return;
      rafId = requestAnimationFrame(() => { update(); rafId = 0; });
    };

    update();
    // Reintento tras el primer paint: por si el layout (alto real del
    // contenedor con scroll) no estaba estable en el montaje inicial.
    retryTimeoutId = window.setTimeout(update, 100);

    scrollEl.addEventListener("scroll", onScroll);
    window.addEventListener("resize", onScroll);
    const resizeObserver = new ResizeObserver(onScroll);
    resizeObserver.observe(scrollEl);

    return () => {
      scrollEl.removeEventListener("scroll", onScroll);
      window.removeEventListener("resize", onScroll);
      resizeObserver.disconnect();
      if (rafId) cancelAnimationFrame(rafId);
      window.clearTimeout(retryTimeoutId);
    };
  }, [itemCount, rowHeight, scrollSelector, overscan]);

  return { containerRef, range };
}
