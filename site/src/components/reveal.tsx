"use client";

import { useEffect } from "react";
import { usePathname } from "next/navigation";

// Everything marked data-reveal starts visible, so the page is complete without scripts. Here,
// what is still below the fold is hidden, then sharpens into place as it scrolls into view.
export function Reveal() {
  const pathname = usePathname();

  useEffect(() => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          observer.unobserve(entry.target);
          (entry.target as HTMLElement).dataset.reveal = "shown";
        }
      },
      { rootMargin: "0px 0px -12% 0px" },
    );
    document.querySelectorAll<HTMLElement>("[data-reveal]").forEach((el) => {
      if (el.getBoundingClientRect().top < window.innerHeight) return;
      el.dataset.reveal = "hidden";
      observer.observe(el);
    });
    return () => observer.disconnect();
  }, [pathname]);

  return null;
}
