"use client";

import { useState } from "react";

const swatches = [
  { name: "Orange", value: "#ff9f0a" },
  { name: "Blue", value: "#3d9bff" },
  { name: "Green", value: "#34c759" },
  { name: "Pink", value: "#ff5c8a" },
  { name: "Violet", value: "#b78cff" },
];

// Dark text on light accents, white text on dark ones (WCAG relative luminance).
function isLight(hex: string) {
  const [r, g, b] = [1, 3, 5].map((i) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.18;
}

// Recolours this page the way Snag's accent setting recolours the app.
export function AccentPicker() {
  const [accent, setAccent] = useState(swatches[0].value);

  function choose(value: string) {
    setAccent(value);
    const root = document.documentElement.style;
    root.setProperty("--accent", value);
    root.setProperty("--on-accent", isLight(value) ? "#1a1816" : "#ffffff");
  }

  return (
    <div role="group" aria-label="Accent colour for this page" className="flex flex-wrap items-center gap-2">
      {swatches.map((swatch) => (
        <button
          key={swatch.value}
          type="button"
          onClick={() => choose(swatch.value)}
          aria-pressed={accent === swatch.value}
          aria-label={swatch.name}
          className="size-9 rounded-full ring-offset-2 ring-offset-panel transition-transform hover:scale-110 aria-pressed:ring-2 aria-pressed:ring-text"
          style={{ background: swatch.value }}
        />
      ))}
      <label className="relative flex h-9 cursor-pointer items-center gap-2 rounded-full bg-raised px-3 text-sm text-muted transition-colors focus-within:outline-2 focus-within:outline-accent hover:text-text">
        <span
          aria-hidden="true"
          className="size-4 rounded-full bg-[conic-gradient(#ff5c5c,#ffd60a,#34c759,#3d9bff,#b78cff,#ff5c5c)]"
        />
        Any colour
        <input
          type="color"
          value={accent}
          onChange={(e) => choose(e.target.value)}
          className="absolute inset-0 cursor-pointer opacity-0"
        />
      </label>
    </div>
  );
}
