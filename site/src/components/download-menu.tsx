"use client";

import { useEffect, useId, useRef, useState, useSyncExternalStore } from "react";
import { AppleLogo, ArrowUpRight, CaretDown, DownloadSimple, LinuxLogo, WindowsLogo } from "@phosphor-icons/react";

import { ExternalLink } from "@/components/external-link";
import { cn } from "@/lib/utils";

type Os = "windows" | "mac" | "linux";

// The visitor's desktop OS, read after hydration; the server (and the first client render)
// always assume Windows, so the markup matches. Phones count as Windows: they can't install
// any build, and the link is what they'd send to their PC.
function readOs(): Os {
  const nav = navigator as Navigator & { userAgentData?: { platform?: string; mobile?: boolean } };
  const platform = (nav.userAgentData?.platform || nav.platform || "").toLowerCase();
  const ua = nav.userAgent.toLowerCase();
  if (nav.userAgentData?.mobile || /android|iphone|ipad|ipod/.test(ua)) return "windows";
  if (platform.includes("mac") || ua.includes("mac os x")) return navigator.maxTouchPoints > 1 ? "windows" : "mac";
  if (platform.includes("linux") || ua.includes("linux") || ua.includes("x11")) return "linux";
  return "windows";
}
const noop = () => () => {};

const sizes = {
  sm: { main: "h-9 px-3.5 text-sm gap-2", chevron: "h-9 w-8", icon: "size-4" },
  lg: { main: "h-12 px-5 text-[0.95rem] gap-2.5", chevron: "h-12 w-11", icon: "size-5" },
};

const align = {
  start: "left-0",
  end: "right-0",
  center: "left-1/2 -translate-x-1/2",
};

// The download button, split: the main part downloads the build for the visitor's system at once,
// the chevron opens every platform. `wholeOnPhone` makes the whole button open the menu on small screens,
// where there is no room for two targets.
export function DownloadMenu({
  downloadUrl,
  releasesUrl,
  version,
  label = "Download for Windows",
  size = "lg",
  menuAlign = "start",
  wholeOnPhone = false,
  className,
}: {
  downloadUrl: string;
  releasesUrl: string;
  version: string | null;
  label?: string;
  size?: keyof typeof sizes;
  menuAlign?: keyof typeof align;
  wholeOnPhone?: boolean;
  className?: string;
}) {
  const os = useSyncExternalStore(noop, readOs, () => "windows" as Os);
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const opener = useRef<HTMLButtonElement | null>(null);
  const focusOnOpen = useRef<"first" | "last" | null>(null);
  const id = useId();
  const s = sizes[size];
  // Every release carries the macOS and Linux builds under stable names too (.github/workflows/release.yml).
  const latest = `${releasesUrl}/latest/download`;
  const builds = {
    windows: { href: downloadUrl, icon: WindowsLogo, label },
    mac: { href: `${latest}/Snag-macOS.dmg`, icon: AppleLogo, label: "Download for macOS" },
    linux: { href: `${latest}/Snag-x86_64.AppImage`, icon: LinuxLogo, label: "Download for Linux" },
  };
  const main = builds[os];
  const MainIcon = main.icon;
  const platforms = [
    { href: downloadUrl, icon: WindowsLogo, name: "Windows 10 & 11", detail: "Installer · .exe" },
    { href: builds.mac.href, icon: AppleLogo, name: "macOS 11+", detail: "Apple silicon & Intel · .dmg" },
    { href: builds.linux.href, icon: LinuxLogo, name: "Linux", detail: "Any distro · AppImage" },
    ...(version
      ? [{ href: `${latest}/snag_${version}_amd64.deb`, icon: LinuxLogo, name: "Ubuntu & Debian", detail: "22.04 or newer · .deb" }]
      : []),
  ];

  const items = () => [...(menu.current?.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? [])];

  function show(trigger: HTMLButtonElement, focus: "first" | "last" | null) {
    opener.current = trigger;
    focusOnOpen.current = focus;
    setOpen(true);
  }

  function close(returnFocus: boolean) {
    setOpen(false);
    if (returnFocus) opener.current?.focus();
  }

  // Focus the first or last item once the menu is on screen.
  useEffect(() => {
    if (!open || !focusOnOpen.current) return;
    const list = items();
    (focusOnOpen.current === "first" ? list[0] : list[list.length - 1])?.focus();
    focusOnOpen.current = null;
  }, [open]);

  // Close on a press anywhere else.
  useEffect(() => {
    if (!open) return;
    const away = (e: PointerEvent) => {
      if (!root.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", away);
    return () => document.removeEventListener("pointerdown", away);
  }, [open]);

  function onTriggerKey(e: React.KeyboardEvent<HTMLButtonElement>) {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      show(e.currentTarget, e.key === "ArrowDown" ? "first" : "last");
    }
  }

  function onTriggerClick(e: React.MouseEvent<HTMLButtonElement>) {
    if (open) return close(false);
    // A click from the keyboard (Enter or Space) moves focus into the menu; a pointer click doesn't.
    show(e.currentTarget, e.detail === 0 ? "first" : null);
  }

  function onMenuKey(e: React.KeyboardEvent<HTMLDivElement>) {
    const list = items();
    const at = list.indexOf(document.activeElement as HTMLElement);
    const go = (i: number) => {
      e.preventDefault();
      list[(i + list.length) % list.length]?.focus();
    };
    if (e.key === "ArrowDown") go(at + 1);
    else if (e.key === "ArrowUp") go(at < 0 ? -1 : at - 1);
    else if (e.key === "Home") go(0);
    else if (e.key === "End") go(-1);
    else if (e.key === "Escape") {
      e.preventDefault();
      close(true);
    } else if (e.key === "Tab") setOpen(false);
    else if (e.key === " " && e.target instanceof HTMLAnchorElement) {
      e.preventDefault();
      e.target.click();
    }
  }

  const triggerProps = {
    type: "button" as const,
    "aria-haspopup": "menu" as const,
    "aria-expanded": open,
    "aria-controls": open ? `${id}-menu` : undefined,
    onClick: onTriggerClick,
    onKeyDown: onTriggerKey,
  };

  const face = "flex items-center justify-center font-semibold whitespace-nowrap transition-colors";
  const fill = "bg-accent text-on-accent hover:bg-accent-hover active:bg-accent-press";

  return (
    <div ref={root} className={cn("relative inline-flex", className)}>
      <div className="dl-button flex w-full overflow-hidden rounded-[4px]">
        <a href={main.href} className={cn(face, fill, s.main, "flex-1", wholeOnPhone && "hidden sm:flex")}>
          <MainIcon weight="fill" className={s.icon} aria-hidden />
          {main.label}
        </a>
        <button
          {...triggerProps}
          id={`${id}-trigger`}
          aria-label="All platforms"
          className={cn(
            face,
            fill,
            s.chevron,
            "shrink-0 shadow-[inset_1px_0_0_rgb(0_0_0/0.22)]",
            wholeOnPhone && "hidden sm:flex",
          )}
        >
          <CaretDown weight="bold" className={cn("size-3.5 transition-transform duration-200", open && "rotate-180")} aria-hidden />
        </button>
        {wholeOnPhone && (
          <button {...triggerProps} className={cn(face, fill, s.main, "flex-1 sm:hidden")}>
            Download
            <CaretDown weight="bold" className={cn("size-3 transition-transform duration-200", open && "rotate-180")} aria-hidden />
          </button>
        )}
      </div>

      {open && (
        <div
          ref={menu}
          id={`${id}-menu`}
          role="menu"
          aria-label="Download Snag"
          onKeyDown={onMenuKey}
          className={cn(
            "dl-menu absolute top-full z-50 mt-2 w-[min(20rem,calc(100vw-2rem))] border border-line-strong bg-bg p-1 text-left shadow-[0_28px_70px_-24px_var(--shadow)]",
            align[menuAlign],
          )}
        >
          <div className="border border-line">
            <p className="label flex items-baseline justify-between px-4 pt-3 pb-2">
              <span>Download Snag</span>
              {version && <span className="normal-case">v{version}</span>}
            </p>
            {platforms.map(({ href, icon: Icon, name, detail }) => (
              <a
                key={href}
                href={href}
                role="menuitem"
                tabIndex={-1}
                onClick={() => setOpen(false)}
                className="group mx-1 flex items-center gap-3 px-3 py-3 text-text outline-none hover:bg-surface-2 focus-visible:bg-surface-2"
              >
                <Icon weight="fill" className="size-6 shrink-0" aria-hidden />
                <span className="min-w-0 flex-1">
                  <span className="block font-medium">{name}</span>
                  <span className="block font-mono text-xs text-faint">{detail}</span>
                </span>
                <DownloadSimple weight="bold" className="size-4 shrink-0 text-accent-text transition-transform group-hover:translate-y-0.5" aria-hidden />
              </a>
            ))}
            <div role="separator" className="mx-4 my-1 h-px bg-line" />
            <ExternalLink
              href={releasesUrl}
              role="menuitem"
              tabIndex={-1}
              onClick={() => setOpen(false)}
              className="mx-1 mb-1 flex items-center justify-between px-3 py-2.5 text-sm text-muted outline-none hover:bg-surface-2 hover:text-text focus-visible:bg-surface-2 focus-visible:text-text"
            >
              All releases on GitHub
              <ArrowUpRight className="size-4" aria-hidden />
            </ExternalLink>
          </div>
        </div>
      )}
    </div>
  );
}
