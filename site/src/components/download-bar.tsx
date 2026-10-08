"use client";

import { useEffect, useRef, useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { CaretDown, CheckCircle, WindowsLogo } from "@phosphor-icons/react";

import { LogoMark } from "@/components/logo";
import { pad, parts } from "@/lib/parts";

// Off the landing page there are no parts to follow, so the eight segments keep pace with the
// scroll on their own: all start together, some race ahead, some lag, all land at 100%.
const pace = [0.7, 1.35, 0.9, 1.7, 1.1, 0.6, 1.5, 1.0];

const clamp = (n: number) => Math.min(1, Math.max(0, n));

// The page is the download, and its index. One segment per part of the page: each fills as you
// read through its part, the label names the part you are in, and opening the label lists them all.
export function DownloadBar({ downloadUrl }: { downloadUrl: string }) {
  const pathname = usePathname();
  const home = pathname === "/";
  const fills = useRef<(HTMLSpanElement | null)[]>([]);
  const counts = useRef<(HTMLSpanElement | null)[]>([]);
  const percent = useRef<HTMLSpanElement>(null);
  const line = useRef<HTMLSpanElement>(null);
  const contents = useRef<HTMLDetailsElement>(null);
  const [done, setDone] = useState(false);
  const [current, setCurrent] = useState(0);
  const [speed, setSpeed] = useState(8.4);

  useEffect(() => {
    let frame = 0;
    const update = () => {
      frame = 0;
      const max = document.documentElement.scrollHeight - window.innerHeight;
      // Within a couple of pixels of the bottom counts as the end: phone toolbars rarely land exactly.
      const finished = max <= 0 || max - window.scrollY <= 2;
      const p = finished ? 1 : clamp(window.scrollY / max);
      const sections = parts.map(({ id }) => document.getElementById(id));

      let progress: number[];
      if (sections.every(Boolean)) {
        // A reading point that runs from the top of the screen (at the top of the page) to its
        // bottom (at the end), so the first part starts empty and the last one ends full.
        const anchor = window.scrollY + window.innerHeight * p;
        const tops = sections.map((el) => el!.getBoundingClientRect().top + window.scrollY);
        tops.push(document.documentElement.scrollHeight);
        progress = parts.map((_, i) => clamp((anchor - tops[i]) / (tops[i + 1] - tops[i])));
        let at = 0;
        tops.slice(0, parts.length).forEach((top, i) => {
          if (anchor >= top) at = i;
        });
        setCurrent(at);
      } else {
        progress = pace.map((k) => p ** k);
      }
      if (finished) progress = progress.map(() => 1);

      progress.forEach((value, i) => {
        const fill = fills.current[i];
        if (fill) fill.style.transform = `scaleX(${value})`;
        const count = counts.current[i];
        if (count) count.textContent = `${Math.round(value * 100)}%`;
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

  // The contents close on a click outside them, or on Escape.
  useEffect(() => {
    const close = (e: Event) => {
      const details = contents.current;
      if (!details?.open) return;
      if (e instanceof KeyboardEvent ? e.key === "Escape" : !details.contains(e.target as Node)) {
        details.open = false;
        if (e instanceof KeyboardEvent) details.querySelector("summary")?.focus();
      }
    };
    document.addEventListener("pointerdown", close);
    document.addEventListener("keydown", close);
    return () => {
      document.removeEventListener("pointerdown", close);
      document.removeEventListener("keydown", close);
    };
  }, []);

  const closeContents = () => {
    if (contents.current) contents.current.open = false;
  };

  return (
    <header
      data-done={done || undefined}
      className="dl-bar sticky top-0 z-40 border-b border-line bg-bg/92 backdrop-blur-md"
    >
      <div className="mx-auto flex h-(--bar-h) max-w-6xl items-center gap-3 px-4 sm:gap-5 sm:px-6">
        <Link href="/" aria-label="Snag home" className="flex shrink-0 items-center gap-2.5">
          <LogoMark size={28} />
          <span className="hidden text-[0.95rem] font-semibold tracking-tight md:inline">Snag</span>
        </Link>

        <div className="min-w-0 flex-1 md:max-w-lg">
          <div className="dl-bar__parts relative flex h-1.5 overflow-hidden" aria-hidden="true">
            {parts.map((part, i) => (
              <span key={part.id} className="hatch-1 h-full flex-1 overflow-hidden bg-surface-2">
                <span
                  ref={(el) => {
                    fills.current[i] = el;
                  }}
                  className="dl-bar__fill block h-full origin-left bg-accent"
                  style={{ transform: "scaleX(0)" }}
                />
              </span>
            ))}
          </div>

          <div className="mt-1 flex items-center justify-between gap-3 font-mono text-[11px]">
            <details ref={contents} className="min-w-0 sm:relative">
              <summary className="flex h-6 cursor-pointer items-center gap-1.5 whitespace-nowrap text-muted hover:text-text">
                {home ? (
                  <>
                    <span className="hidden sm:inline">Part</span>
                    <span>
                      <span className="text-text">{pad(current + 1)}</span>
                      <span className="sm:hidden">/</span>
                      <span className="hidden sm:inline"> of </span>
                      {pad(parts.length)} ·
                    </span>
                    <span key={current} className="dl-bar__name truncate text-text">
                      {parts[current].name}
                    </span>
                  </>
                ) : (
                  <span className="text-text">Contents</span>
                )}
                <CaretDown weight="bold" className="size-3 shrink-0" aria-hidden />
              </summary>
              <nav
                aria-label="Contents"
                className="dl-bar__contents fixed inset-x-4 top-[calc(var(--bar-h)+0.5rem)] border border-line-strong bg-bg p-1 shadow-[0_24px_60px_-20px_rgb(0_0_0/0.8)] sm:absolute sm:inset-x-auto sm:top-full sm:left-0 sm:mt-2 sm:w-80"
              >
                <div className="border border-line py-2">
                  <p className="label px-4 pt-1 pb-2">Contents · 8 parts</p>
                  <ol>
                    {parts.map((part, i) => (
                      <li key={part.id}>
                        <Link
                          href={home ? `#${part.id}` : `/#${part.id}`}
                          onClick={closeContents}
                          aria-current={home && i === current ? "location" : undefined}
                          className="flex items-baseline gap-3 px-4 py-2 text-muted hover:bg-surface-2 hover:text-text aria-[current]:text-text"
                        >
                          <span className="w-5 text-faint">{pad(i + 1)}</span>
                          <span className="font-display text-base italic">{part.name}</span>
                          <span aria-hidden="true" className="leader" />
                          <span
                            ref={(el) => {
                              counts.current[i] = el;
                            }}
                            className={home ? "text-faint tabular-nums" : "hidden"}
                          >
                            0%
                          </span>
                        </Link>
                      </li>
                    ))}
                  </ol>
                </div>
              </nav>
            </details>

            {done ? (
              <Link
                href="/#get-started"
                className="dl-bar__done flex h-6 shrink-0 items-center gap-1.5 text-accent hover:brightness-110"
              >
                <CheckCircle weight="fill" className="size-3.5" aria-hidden />
                Done<span className="hidden sm:inline"> · Open</span>
              </Link>
            ) : (
              <span className="flex shrink-0 gap-3 text-faint tabular-nums" aria-hidden="true">
                <span ref={percent} className="text-text">
                  0%
                </span>
                <span className="hidden w-[4.5rem] sm:inline">{speed.toFixed(1)} MB/s</span>
              </span>
            )}
          </div>
        </div>

        <a
          href={downloadUrl}
          className="ml-auto flex h-9 shrink-0 items-center gap-2 rounded-[3px] bg-accent px-3.5 text-sm font-semibold text-on-accent transition-[filter] hover:brightness-110 sm:px-4"
        >
          <WindowsLogo weight="fill" className="size-4" aria-hidden />
          Download
        </a>
      </div>
      {/* The whole page's progress along the bar's bottom edge. */}
      <span
        ref={line}
        aria-hidden="true"
        className="dl-bar__line absolute -bottom-px left-0 h-px w-full origin-left bg-accent"
        style={{ transform: "scaleX(0)" }}
      />
    </header>
  );
}
