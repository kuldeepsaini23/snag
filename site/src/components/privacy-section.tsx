import Link from "next/link";
import {
  Bell,
  Browser,
  Clipboard,
  Cookie,
  DeviceMobile,
  Gear,
  Globe,
  HardDrives,
  ShieldCheck,
} from "@phosphor-icons/react/dist/ssr";

import { LogoMark } from "@/components/logo";
import { cn } from "@/lib/utils";

const switches = [
  { icon: Clipboard, title: "Read copied links", body: "Only links: a copied link pops up in the corner, ready to download.", on: true },
  { icon: Bell, title: "Windows notifications", body: "When a download finishes or fails.", on: true },
  { icon: DeviceMobile, title: "Phone sharing on your Wi-Fi", body: "Send links from your phone. Off until you turn it on.", on: false },
];

const facts = [
  { icon: Cookie, title: "Browser cookies", body: "Only for the site you download from, only when you send the download." },
  { icon: Globe, title: "Internet", body: "Only the sites you download from, plus tool updates from GitHub. Nothing about you is sent." },
  { icon: ShieldCheck, title: "Safety check", body: "Off until you add your own VirusTotal key. Only a fingerprint is sent." },
];

function Toggle({ on }: { on: boolean }) {
  return (
    <span
      aria-hidden="true"
      className={cn("relative h-5 w-9 shrink-0 rounded-full transition-colors", on ? "bg-accent" : "bg-selected")}
    >
      <span className={cn("absolute top-0.5 size-4 rounded-full bg-text", on ? "right-0.5" : "left-0.5")} />
    </span>
  );
}

function Node({
  icon: Icon,
  title,
  detail,
  accent,
}: {
  icon?: typeof Globe;
  title: string;
  detail: string;
  accent?: boolean;
}) {
  return (
    <div
      className={cn(
        "flex items-center gap-3 rounded-xl bg-raised px-4 py-3",
        accent && "shadow-[0_0_0_1.5px_var(--accent)]",
      )}
    >
      {Icon ? <Icon className="size-6 shrink-0 text-text" aria-hidden /> : <LogoMark size={28} />}
      <div className="min-w-0">
        <p className="text-sm font-semibold">{title}</p>
        <p className="font-mono text-[11px] leading-snug text-faint">{detail}</p>
      </div>
    </div>
  );
}

function Wire({ className }: { className?: string }) {
  return <span aria-hidden="true" className={cn("wire mx-auto block h-8 w-0.5", className)} />;
}

export function PrivacySection() {
  return (
    <section id="privacy" className="mx-auto max-w-6xl px-4 pt-24 sm:px-6 md:pt-32">
      <h2 className="text-4xl font-bold tracking-tight md:text-5xl">Everything stays on your PC</h2>
      <p className="mt-4 max-w-[56ch] text-lg leading-relaxed text-muted">
        No account, no analytics, no server of our own. Your downloads, settings and history stay in{" "}
        <code className="font-mono text-[0.9em] text-text">%APPDATA%\Snag</code>.
      </p>

      <div className="mt-10 grid gap-5 lg:grid-cols-[1.15fr_1fr]">
        {/* Styled after the last card of Snag's first-run tour. */}
        <div className="rounded-2xl bg-panel p-5 shadow-[0_0_0_1px_var(--line)] sm:p-6">
          <div className="flex items-center gap-4">
            <span className="grid size-12 shrink-0 place-items-center rounded-xl bg-accent/15">
              <Gear className="size-6 text-accent" aria-hidden />
            </span>
            <div>
              <h3 className="text-lg font-semibold">You decide what Snag may use</h3>
              <p className="text-sm text-muted">Change any of these later in Settings.</p>
            </div>
          </div>
          <ul className="mt-5 divide-y divide-line rounded-xl bg-raised/60">
            {switches.map(({ icon: Icon, title, body, on }) => (
              <li key={title} className="flex items-center gap-3 px-4 py-3">
                <Icon className="size-5 shrink-0 text-accent" aria-hidden />
                <div className="min-w-0 flex-1">
                  <p className="text-sm font-medium">{title}</p>
                  <p className="text-xs leading-relaxed text-faint">{body}</p>
                </div>
                <Toggle on={on} />
                <span className="sr-only">{on ? "on by default" : "off by default"}</span>
              </li>
            ))}
          </ul>
          <ul className="mt-3 divide-y divide-line rounded-xl bg-raised/60">
            {facts.map(({ icon: Icon, title, body }) => (
              <li key={title} className="flex items-start gap-3 px-4 py-3">
                <Icon className="mt-0.5 size-5 shrink-0 text-accent" aria-hidden />
                <div>
                  <p className="text-sm font-medium">{title}</p>
                  <p className="text-xs leading-relaxed text-faint">{body}</p>
                </div>
              </li>
            ))}
          </ul>
        </div>

        {/* Every connection Snag makes. There is no line to a Snag server because there isn't one. */}
        <div className="flex flex-col justify-center rounded-2xl bg-panel p-5 shadow-[0_0_0_1px_var(--line)] sm:p-6">
          <p className="text-sm font-semibold">Where your data goes</p>
          <p className="mt-1 text-sm text-faint">The extension talks only to Snag on this computer, after you press Allow.</p>
          <div
            className="mt-6"
            role="img"
            aria-label="The browser extension and the sites you download from connect only to Snag on your PC at 127.0.0.1, which saves files to your own disk."
          >
            <div className="grid grid-cols-2 gap-3">
              <Node icon={Browser} title="Extension" detail="in your browser" />
              <Node icon={Globe} title="Sites" detail="you download from" />
            </div>
            <div className="grid grid-cols-2">
              <Wire />
              <Wire />
            </div>
            <div className="mx-auto max-w-xs">
              <Node title="Snag" detail="127.0.0.1 · this PC" accent />
            </div>
            <Wire />
            <div className="mx-auto max-w-xs">
              <Node icon={HardDrives} title="Your disk" detail="Downloads · %APPDATA%\Snag" />
            </div>
          </div>
          <p className="mt-6 text-center font-mono text-xs text-faint">Snag servers: none</p>
        </div>
      </div>

      <figure className="mt-5 overflow-hidden rounded-2xl bg-panel shadow-[0_0_0_1px_var(--line)]">
        <picture>
          <source
            type="image/webp"
            srcSet="/images/permissions-760.webp 760w, /images/permissions-1382.webp 1382w"
            sizes="(min-width: 1152px) 1104px, calc(100vw - 32px)"
          />
          <img
            src="/images/permissions-1382.webp"
            alt="Snag's first-run tour: switches for copied links, notifications and phone sharing, and what Snag does with cookies, the internet and the safety check"
            width={1382}
            height={864}
            loading="lazy"
            decoding="async"
            className="block h-auto w-full"
          />
        </picture>
        <figcaption className="flex flex-wrap items-center justify-between gap-3 px-5 py-4 text-sm text-muted">
          The real thing: the first time you open Snag, it asks.
          <Link href="/privacy/" className="font-semibold text-accent underline decoration-accent/40 underline-offset-4 hover:decoration-accent">
            Read the privacy policy
          </Link>
        </figcaption>
      </figure>
    </section>
  );
}
