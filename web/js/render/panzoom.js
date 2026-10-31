// Pan and zoom for an SVG viewport group: drag to pan, wheel or pinch to
// zoom, and an animated fit. Taps (a press without movement) pass through.

const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));

export function panzoom(svg, group, { min = 0.12, max = 3 } = {}) {
  const t = { x: 0, y: 0, k: 1 };
  const apply = () => group.setAttribute("transform", `translate(${t.x},${t.y}) scale(${t.k})`);
  const zoomAt = (px, py, k) => {
    k = clamp(k, min, max);
    t.x = px - ((px - t.x) * k) / t.k;
    t.y = py - ((py - t.y) * k) / t.k;
    t.k = k;
    apply();
  };
  const local = (e) => {
    const r = svg.getBoundingClientRect();
    return [e.clientX - r.left, e.clientY - r.top];
  };

  svg.addEventListener("wheel", (e) => {
    e.preventDefault();
    const [x, y] = local(e);
    zoomAt(x, y, t.k * Math.exp(-e.deltaY * 0.0015));
  }, { passive: false });

  const pointers = new Map();
  let pinch = null;
  let moved = 0;
  svg.addEventListener("pointerdown", (e) => {
    pointers.set(e.pointerId, local(e));
    moved = 0;
    if (pointers.size === 2) {
      const [a, b] = [...pointers.values()];
      pinch = { d: Math.hypot(a[0] - b[0], a[1] - b[1]), k: t.k };
    }
  });
  svg.addEventListener("pointermove", (e) => {
    const prev = pointers.get(e.pointerId);
    if (!prev) return;
    const now = local(e);
    pointers.set(e.pointerId, now);
    if (pinch && pointers.size === 2) {
      const [a, b] = [...pointers.values()];
      zoomAt((a[0] + b[0]) / 2, (a[1] + b[1]) / 2, (pinch.k * Math.hypot(a[0] - b[0], a[1] - b[1])) / pinch.d);
    } else {
      moved += Math.abs(now[0] - prev[0]) + Math.abs(now[1] - prev[1]);
      if (moved > 4) svg.setPointerCapture?.(e.pointerId);
      t.x += now[0] - prev[0];
      t.y += now[1] - prev[1];
      apply();
    }
  });
  const end = (e) => {
    pointers.delete(e.pointerId);
    if (pointers.size < 2) pinch = null;
  };
  svg.addEventListener("pointerup", end);
  svg.addEventListener("pointercancel", end);

  let anim = 0;
  return {
    /** Whether the last press was a drag (so it is not a tap). */
    dragged: () => moved > 4,
    /** Animate to fit `box` ({x, y, w, h}); `minK` keeps text readable. */
    fit(box, { pad = 28, minK = 0, duration = 450, anchor = "center" } = {}) {
      const W = svg.clientWidth, H = svg.clientHeight;
      if (!W || !H || !box) return;
      const k = clamp(Math.max(minK, Math.min((W - 2 * pad) / box.w, (H - 2 * pad) / box.h)), min, 1.4);
      const cx = box.x + box.w / 2;
      const to = { k, x: W / 2 - cx * k, y: anchor === "bottom" && box.h * k > H ? H - pad - (box.y + box.h) * k : H / 2 - (box.y + box.h / 2) * k };
      const from = { ...t };
      const start = performance.now();
      cancelAnimationFrame(anim);
      const step = (now) => {
        const p = Math.min(1, (now - start) / duration);
        const e = p < 0.5 ? 4 * p * p * p : 1 - (-2 * p + 2) ** 3 / 2;
        for (const key of ["x", "y", "k"]) t[key] = from[key] + (to[key] - from[key]) * e;
        apply();
        if (p < 1) anim = requestAnimationFrame(step);
      };
      anim = requestAnimationFrame(step);
    },
    center(x, y, duration = 300) {
      const W = svg.clientWidth, H = svg.clientHeight;
      this.fit({ x: x - W / 2 / t.k, y: y - H / 2 / t.k, w: W / t.k, h: H / t.k }, { pad: 0, minK: t.k, duration });
    },
  };
}
