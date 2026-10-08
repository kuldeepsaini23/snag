"use client";

import { useEffect, useSyncExternalStore } from "react";
import { CircleHalf } from "@phosphor-icons/react";

import { THEME_KEY, type Theme } from "@/lib/theme";
import { cn } from "@/lib/utils";

// The printing in use is whatever <html data-theme> says (the inline script in layout.tsx sets it
// before paint); the server renders Ink and the client catches up after hydration.
function subscribe(onChange: () => void) {
  const observer = new MutationObserver(onChange);
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
  return () => observer.disconnect();
}
const read = (): Theme => (document.documentElement.dataset.theme === "paper" ? "paper" : "ink");

function stored(): Theme | null {
  try {
    const value = localStorage.getItem(THEME_KEY);
    return value === "ink" || value === "paper" ? value : null;
  } catch {
    return null;
  }
}

function apply(theme: Theme) {
  const set = () => {
    document.documentElement.dataset.theme = theme;
  };
  // A soft cross-fade where the browser can do one, unless motion is reduced.
  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (!reduce && "startViewTransition" in document) document.startViewTransition(set);
  else set();
}

export function useTheme() {
  const theme = useSyncExternalStore(subscribe, read, () => "ink" as Theme);

  // Until the visitor picks one, follow the system as it changes (e.g. at sunset).
  useEffect(() => {
    const query = window.matchMedia("(prefers-color-scheme: light)");
    const follow = () => {
      if (!stored()) apply(query.matches ? "paper" : "ink");
    };
    query.addEventListener("change", follow);
    return () => query.removeEventListener("change", follow);
  }, []);

  const choose = (next: Theme) => {
    try {
      localStorage.setItem(THEME_KEY, next);
    } catch {}
    apply(next);
  };
  return [theme, choose] as const;
}

// The bar's switch: one round button, the half-inked circle turning over with the page.
export function ThemeToggle({ className }: { className?: string }) {
  const [theme, choose] = useTheme();
  const paper = theme === "paper";
  return (
    <button
      type="button"
      onClick={() => choose(paper ? "ink" : "paper")}
      aria-pressed={paper}
      aria-label="Light page (Paper)"
      title={paper ? "Paper printing · switch to Ink" : "Ink printing · switch to Paper"}
      className={cn(
        "flex size-9 items-center justify-center rounded-full text-muted shadow-[inset_0_0_0_1px_var(--hairline)] transition-[color,box-shadow] hover:text-text hover:shadow-[inset_0_0_0_1px_var(--accent)]",
        className,
      )}
    >
      <CircleHalf
        weight="fill"
        className={cn("size-[18px] transition-transform duration-500", paper ? "rotate-180" : "rotate-0")}
        aria-hidden
      />
    </button>
  );
}

// The same choice as two words, for the contents panel (where phones find it).
export function ThemeChoice({ className }: { className?: string }) {
  const [theme, choose] = useTheme();
  return (
    <div role="group" aria-label="Page printing" className={cn("flex items-center gap-3", className)}>
      <span className="label">Printing</span>
      <span aria-hidden="true" className="leader" />
      {(["ink", "paper"] as const).map((t) => (
        <button
          key={t}
          type="button"
          onClick={() => choose(t)}
          aria-pressed={theme === t}
          className="min-h-6 font-display text-base text-faint capitalize italic transition-colors hover:text-text aria-pressed:text-text aria-pressed:underline aria-pressed:decoration-accent aria-pressed:underline-offset-4"
        >
          {t}
        </button>
      ))}
    </div>
  );
}
