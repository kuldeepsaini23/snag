"use client";

import { useEffect } from "react";

// Download rows start finished, so the page is complete without scripts. Here, rows still
// below the fold are queued, then "download" when they scroll into view: the bar runs for
// the row's --run time, the status flips to Done, and the row opens.
export function RowObserver() {
  useEffect(() => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const timers: number[] = [];
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          const row = entry.target as HTMLElement;
          observer.unobserve(row);
          row.dataset.state = "downloading";
          const run = parseFloat(getComputedStyle(row).getPropertyValue("--run")) || 1.4;
          timers.push(window.setTimeout(() => (row.dataset.state = "done"), run * 1000 + 150));
        }
      },
      { threshold: 0.6 },
    );
    document.querySelectorAll<HTMLElement>(".dl-row").forEach((row) => {
      if (row.getBoundingClientRect().top < window.innerHeight) return;
      row.dataset.state = "queued";
      observer.observe(row);
    });
    return () => {
      observer.disconnect();
      timers.forEach(clearTimeout);
    };
  }, []);

  return null;
}
