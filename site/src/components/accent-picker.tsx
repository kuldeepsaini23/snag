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

// Recolours this page the way Snag's accent setting recolours the app. Set out like the chips
// of a paint catalogue: a swatch with its name under it.
export function AccentPicker() {
  const [accent, setAccent] = useState(swatches[0].value);

  function choose(value: string) {
    setAccent(value);
    const root = document.documentElement.style;
    root.setProperty("--accent", value);
    root.setProperty("--on-accent", isLight(value) ? "#0b0b0c" : "#ffffff");
  }

  return (
    <div role="group" aria-label="Accent colour for this page" className="flex flex-wrap gap-x-2.5 gap-y-4 sm:gap-x-3">
      {swatches.map((swatch) => (
        <button
          key={swatch.value}
          type="button"
          onClick={() => choose(swatch.value)}
          aria-pressed={accent === swatch.value}
          className="group flex w-12 flex-col items-center sm:w-14 gap-2 font-mono text-[11px] text-faint hover:text-text aria-pressed:text-text"
        >
          <span
            aria-hidden="true"
            className="block size-10 sm:size-12 shadow-[0_0_0_1px_var(--line-strong)] transition-[box-shadow,translate] group-hover:-translate-y-0.5 group-aria-pressed:shadow-[0_0_0_3px_var(--bg),0_0_0_4px_var(--text)]"
            style={{ background: swatch.value }}
          />
          {swatch.name}
        </button>
      ))}
      <label className="group relative flex w-12 cursor-pointer flex-col sm:w-14 items-center gap-2 font-mono text-[11px] text-faint focus-within:outline-2 focus-within:outline-offset-4 focus-within:outline-accent hover:text-text">
        <span
          aria-hidden="true"
          className="block size-10 sm:size-12 bg-[conic-gradient(#ff5c5c,#ffd60a,#34c759,#3d9bff,#b78cff,#ff5c5c)] shadow-[0_0_0_1px_var(--line-strong)] transition-[translate] group-hover:-translate-y-0.5"
        />
        Any
        <input
          type="color"
          value={accent}
          onChange={(e) => choose(e.target.value)}
          aria-label="Any colour"
          className="absolute inset-0 cursor-pointer opacity-0"
        />
      </label>
    </div>
  );
}
