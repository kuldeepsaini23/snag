"use client";

import { useEffect, useState } from "react";

import { useActiveSection } from "@/lib/use-active-section";
import { cn } from "@/lib/utils";

const ticks = Array.from({ length: 21 }, (_, i) => i * 5);

// Desktop only: the page's sections with the current one large, and a ruler marking how far down you are.
export function ScrollRail({ sections }: { sections: readonly { id: string; label: string }[] }) {
  const active = useActiveSection(sections.map((s) => s.id)) ?? sections[0].id;
  const [progress, setProgress] = useState(0);

  useEffect(() => {
    let frame = 0;
    const update = () => {
      frame = 0;
      const max = document.documentElement.scrollHeight - window.innerHeight;
      setProgress(max > 0 ? Math.min(1, Math.max(0, window.scrollY / max)) : 0);
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
  }, []);

  const pct = Math.round(progress * 100);

  return (
    <aside
      aria-label="Page sections"
      className="fixed top-(--header-h) right-0 bottom-0 z-30 hidden w-(--rail-w) border-l border-line lg:block"
    >
      <nav className="absolute right-13 bottom-8 left-3">
        <ul className="flex flex-col items-end gap-1.5 text-right">
          {sections.map((s) => (
            <li key={s.id}>
              <a
                href={`#${s.id}`}
                aria-current={active === s.id ? "true" : undefined}
                className={cn(
                  "block font-display leading-none font-bold whitespace-nowrap uppercase transition-[font-size,color] duration-300 [font-stretch:125%]",
                  active === s.id ? "text-[1.15rem] text-text" : "text-[0.78rem] text-faint hover:text-muted",
                )}
              >
                {s.label}
              </a>
            </li>
          ))}
        </ul>
      </nav>

      <div aria-hidden="true" className="absolute top-3 right-0 bottom-3 w-12">
        {ticks.map((t) => (
          <div key={t} className="absolute right-0 flex -translate-y-1/2 items-center gap-1.5" style={{ top: `${t}%` }}>
            {t % 10 === 0 && <span className="font-mono text-[9px] leading-none text-faint">{t}</span>}
            <span className={cn("h-px bg-line-strong", t % 10 === 0 ? "w-3" : "w-1.5")} />
          </div>
        ))}
        <div
          className="absolute right-0 flex -translate-y-1/2 items-center gap-1 bg-bg pl-1"
          style={{ top: `${pct}%` }}
        >
          <span className="font-mono text-[9px] leading-none text-accent">{String(pct).padStart(3, "0")}%</span>
          <span className="h-px w-5 bg-accent" />
        </div>
      </div>
    </aside>
  );
}
