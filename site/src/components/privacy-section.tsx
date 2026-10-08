import Link from "next/link";
import {
  Bell,
  Browser,
  Clipboard,
  Cookie,
  DeviceMobile,
  Globe,
  HardDrives,
  ShieldCheck,
} from "@phosphor-icons/react/dist/ssr";

import { Band, Plate, Section, SectionHead, Shot } from "@/components/catalogue";
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
      className={cn(
        "relative h-5 w-9 shrink-0 rounded-full",
        on ? "bg-accent" : "bg-surface-2 shadow-[inset_0_0_0_1px_var(--line-strong)]",
      )}
    >
      <span className={cn("absolute top-0.5 size-4 rounded-full", on ? "right-0.5 bg-text" : "left-0.5 bg-faint")} />
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
    <div className={cn("border p-[3px]", accent ? "border-text" : "border-line-strong")}>
      <div className={cn("flex items-center gap-3 border px-3.5 py-3", accent ? "border-text bg-bg" : "border-line bg-bg")}>
        {Icon ? <Icon className="size-6 shrink-0" aria-hidden /> : <LogoMark size={28} />}
        <div className="min-w-0">
          <p className="text-sm font-semibold">{title}</p>
          <p className="font-mono text-[11px] leading-snug text-faint">{detail}</p>
        </div>
      </div>
    </div>
  );
}

function Wire({ className }: { className?: string }) {
  return <span aria-hidden="true" className={cn("wire mx-auto block h-9 w-px", className)} />;
}

// The one part printed the other way round (paper on the Ink page, ink on the Paper one), with
// engraved hatching, like a notice pasted into the catalogue.
export function PrivacySection() {
  return (
    <Section id="privacy" className="contrast mt-28 overflow-hidden md:mt-40">
      <Band className="mt-3" />
      {/* Hatching and a rosette in the margin: the engraver's shading on the notice. */}
      <div
        aria-hidden="true"
        className="hatch pointer-events-none absolute inset-x-0 top-12 h-56 [mask-image:linear-gradient(to_bottom,black,transparent)]"
      />
      <div aria-hidden="true" className="rosette pointer-events-none absolute -top-24 -right-40 size-[34rem] md:-right-24" />

      <div className="relative mx-auto max-w-6xl px-4 pt-16 pb-24 sm:px-6 md:pt-24 md:pb-32">
        <SectionHead
          id="privacy"
          title={
            <>
              Everything stays on <em>your</em> PC.
            </>
          }
          intro={
            <>
              No account, no analytics, no server of our own. Your downloads, settings and history stay in{" "}
              <code className="font-mono text-[0.9em] text-text">%APPDATA%\Snag</code>.
            </>
          }
        />

        <div className="mt-14 grid gap-x-14 gap-y-16 md:mt-20 lg:grid-cols-[1.1fr_1fr]">
          {/* Styled after the last card of Snag's first-run tour. */}
          <div data-reveal>
            <h3 className="font-display text-3xl italic">You decide what Snag may use</h3>
            <p className="mt-2 text-muted">Change any of these later in Settings.</p>
            <ul className="mt-6 border-t border-text">
              {switches.map(({ icon: Icon, title, body, on }) => (
                <li key={title} className="flex items-center gap-4 border-b border-line py-4">
                  <Icon className="size-5 shrink-0" aria-hidden />
                  <div className="min-w-0 flex-1">
                    <p className="font-medium">{title}</p>
                    <p className="text-sm leading-relaxed text-muted">{body}</p>
                  </div>
                  <Toggle on={on} />
                  <span className="sr-only">{on ? "on by default" : "off by default"}</span>
                </li>
              ))}
            </ul>
            <ul className="mt-8 border-t border-text">
              {facts.map(({ icon: Icon, title, body }) => (
                <li key={title} className="flex items-start gap-4 border-b border-line py-4">
                  <Icon className="mt-0.5 size-5 shrink-0" aria-hidden />
                  <div>
                    <p className="font-medium">{title}</p>
                    <p className="text-sm leading-relaxed text-muted">{body}</p>
                  </div>
                </li>
              ))}
            </ul>
          </div>

          {/* Every connection Snag makes. There is no line to a Snag server because there isn't one. */}
          <div data-reveal>
            <h3 className="font-display text-3xl italic">Where your data goes</h3>
            <p className="mt-2 text-muted">The extension talks only to Snag on this computer, after you press Allow.</p>
            <div
              className="relative mt-8"
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
            {/* Set like a rubber stamp on the notice. */}
            <p className="mx-auto mt-10 w-fit -rotate-3 border-[3px] border-double border-accent-text px-4 py-1.5 font-mono text-sm tracking-[0.18em] text-accent-text uppercase">
              Snag servers: none
            </p>
          </div>
        </div>

        <div className="mt-20 grid items-center gap-x-14 gap-y-8 md:grid-cols-[minmax(0,26rem)_1fr]" data-reveal>
          <Plate window fig={5} caption="the first-run tour’s last card.">
            <Shot
              name="permissions"
              widths={[580]}
              height={656}
              sizes="(min-width: 768px) 416px, calc(100vw - 44px)"
              alt="Snag’s first-run tour: switches for copied links, notifications and phone sharing, and what Snag does with cookies, the internet and the safety check"
            />
          </Plate>
          <div>
            <p className="font-display text-[clamp(2rem,6vw,3.25rem)] leading-[1.05] tracking-tight italic">
              The real thing: the first time you open Snag, it asks.
            </p>
            <Link
              href="/privacy/"
              prefetch={false}
              className="mt-8 inline-flex items-center gap-2 border-b border-text pb-1 font-semibold hover:border-accent-text hover:text-accent-text"
            >
              Read the privacy policy →
            </Link>
          </div>
        </div>
      </div>
      <Band className="mb-3" />
    </Section>
  );
}
