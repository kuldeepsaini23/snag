"use client";

import { useEffect, useRef, useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { CheckCircle, WindowsLogo } from "@phosphor-icons/react";

import { LogoMark } from "@/components/logo";

// How each of the eight parts keeps pace with the scroll: all start together, some race
// ahead, some lag, and every one lands at 100% at the bottom of the page.
const pace = [0.7, 1.35, 0.9, 1.7, 1.1, 0.6, 1.5, 1.0];

// The page is the download: this bar fills as you scroll and finishes at the bottom.
export function DownloadBar({ downloadUrl }: { downloadUrl: string }) {
  const pathname = usePathname();
  const parts = useRef<(HTMLSpanElement | null)[]>([]);
  const percent = useRef<HTMLSpanElement>(null);
  const line = useRef<HTMLSpanElement>(null);
  const [done, setDone] = useState(false);
  const [speed, setSpeed] = useState(8.4);

  useEffect(() => {
    let frame = 0;
    const update = () => {
      frame = 0;
      const max = document.documentElement.scrollHeight - window.innerHeight;
      // Within a couple of pixels of the bottom counts as the end: phone toolbars rarely land exactly.
      const finished = max <= 0 || max - window.scrollY <= 2;
      const p = finished ? 1 : Math.min(1, Math.max(0, window.scrollY / max));
      parts.current.forEach((el, i) => {
        if (el) el.style.transform = `scaleX(${p ** pace[i]})`;
      });
      if (line.current) line.current.style.transform = `scaleX(${p})`;
      if (percent.current) percent.current.textContent = `${Math.min(99, Math.round(p * 100))}%`;
      setDone(finished);
    };
    const onScroll = () => {
      if (!frame) frame = requestAnimationFrame(update);
    };
    update();
    window.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", onScroll);
    return () => {
      window.removeEventListener("scroll", onScroll);
      window.removeEventListener("resize", onScroll);
      cancelAnimationFrame(frame);
    };
  }, [pathname]);

  // A believable speed that drifts a little, like the real one.
  useEffect(() => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const timer = setInterval(() => setSpeed(Math.round((7.6 + Math.random() * 1.7) * 10) / 10), 1100);
    return () => clearInterval(timer);
  }, []);

  return (
    <header
      data-done={done || undefined}
      className="dl-bar sticky top-0 z-40 border-b border-line bg-panel/90 backdrop-blur-md"
    >
      <div className="mx-auto flex h-(--bar-h) max-w-6xl items-center gap-3 px-4 sm:gap-4 sm:px-6">
        <Link href="/" aria-label="Snag home" className="flex shrink-0 items-center gap-3 rounded-lg">
          <LogoMark size={30} />
          <span className="hidden min-w-0 flex-col leading-tight md:flex">
            <span className="text-sm font-semibold">
              snag.kuldeepsaini.dev{pathname === "/" ? "" : pathname.replace(/\/$/, "")}
            </span>
            <span className="font-mono text-[11px] text-faint">
              {done ? "All 8 parts done" : "8 parallel connections"}
            </span>
          </span>
        </Link>

        <div className="flex min-w-0 flex-1 items-center gap-3" aria-hidden="true">
          <div className="dl-bar__parts relative flex h-2.5 min-w-0 flex-1 overflow-hidden rounded-[3px] md:max-w-sm">
            {pace.map((_, i) => (
              <span key={i} className="h-full flex-1 overflow-hidden bg-raised">
                <span
                  ref={(el) => {
                    parts.current[i] = el;
                  }}
                  className="dl-bar__fill block h-full origin-left bg-accent"
                  style={{ transform: "scaleX(0)" }}
                />
              </span>
            ))}
          </div>
          <span className={`shrink-0 items-baseline gap-3 font-mono text-xs tabular-nums ${done ? "hidden" : "flex"}`}>
            <span ref={percent} className="w-9 text-right text-text">
              0%
            </span>
            <span className="hidden w-[4.5rem] text-faint sm:inline">{speed.toFixed(1)} MB/s</span>
          </span>
        </div>

        {done && (
          <Link
            href="/#get-started"
            className="dl-bar__done flex shrink-0 items-center gap-1.5 rounded-full bg-accent/15 px-2.5 py-1 font-mono text-xs text-accent hover:bg-accent/25"
          >
            <CheckCircle weight="fill" className="size-4" aria-hidden />
            Done<span className="hidden sm:inline"> · Open</span>
          </Link>
        )}

        <a
          href={downloadUrl}
          className="flex h-9 shrink-0 items-center gap-2 rounded-full bg-accent px-4 text-sm font-semibold text-on-accent transition-[filter] hover:brightness-110"
        >
          <WindowsLogo weight="fill" className="size-4" aria-hidden />
          Download
        </a>
      </div>
      {/* The whole page's progress along the bar's bottom edge. */}
      <span
        ref={line}
        aria-hidden="true"
        className="dl-bar__line absolute -bottom-px left-0 h-0.5 w-full origin-left bg-accent"
        style={{ transform: "scaleX(0)" }}
      />
    </header>
  );
}
