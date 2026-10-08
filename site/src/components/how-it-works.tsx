import { CursorClick, DownloadSimple, Plug } from "@phosphor-icons/react/dist/ssr";

import { Plate, Section, SectionHead, Shot } from "@/components/catalogue";
import { INSTALLER_NAME } from "@/lib/site";

const steps = [
  {
    numeral: "I",
    icon: DownloadSimple,
    title: "Install Snag",
    body: "Run the installer. It sets Snag up for your Windows user only, so it needs no admin rights.",
    hint: `${INSTALLER_NAME} · 12 MB`,
  },
  {
    numeral: "II",
    icon: Plug,
    title: "Connect the browser extension",
    body: "Click the Snag icon in your browser, then Connect to Snag, and press Allow in Snag. You do this once.",
    hint: "One click, then Allow",
  },
  {
    numeral: "III",
    icon: CursorClick,
    title: "Click Download on any video",
    body: "A Download button appears on videos as they play. Pick a quality and Snag takes it from there.",
    hint: "1080p · 720p · MP3",
  },
];

const stores = [
  { browser: "Chrome", store: "Chrome Web Store" },
  { browser: "Edge", store: "Edge Add-ons" },
  { browser: "Firefox", store: "Firefox Add-ons" },
];

const code = "bg-surface-2 px-1.5 py-0.5 font-mono text-[0.85em] text-text";

export function HowItWorks() {
  return (
    <Section id="how-it-works" className="mx-auto max-w-6xl px-4 pt-28 sm:px-6 md:pt-40">
      <SectionHead
        id="how-it-works"
        title={
          <>
            Set up in a <em>minute</em>.
          </>
        }
      />

      {/* Three steps across the page, each under its roman numeral, ruled apart like columns of type. */}
      <ol className="mt-14 grid border-y border-line md:mt-20 md:grid-cols-3 md:divide-x md:divide-line">
        {steps.map((step, i) => {
          const Icon = step.icon;
          return (
            <li
              key={step.title}
              className="flex flex-col border-line py-8 not-first:border-t md:px-8 md:py-10 md:not-first:border-t-0 md:first:pl-0 md:last:pr-0"
              data-reveal
              style={{ "--reveal-delay": `${i * 0.12}s` } as React.CSSProperties}
            >
              <div className="flex items-start justify-between">
                <span className="font-display text-6xl leading-none font-light italic">{step.numeral}.</span>
                <Icon className="size-6 text-accent-text" aria-hidden />
              </div>
              <h3 className="mt-6 text-xl font-semibold tracking-tight">{step.title}</h3>
              <p className="mt-2 leading-relaxed text-muted">{step.body}</p>
              <p className="mt-auto pt-6 font-mono text-xs text-faint">{step.hint}</p>
            </li>
          );
        })}
      </ol>

      <div id="extension" className="mt-20 grid gap-x-14 gap-y-12 md:mt-28 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.05fr)]">
        <div data-reveal>
          <h3 className="font-display text-3xl italic md:text-4xl">The browser extension</h3>
          <ul className="mt-6 border-t border-text">
            {stores.map((s) => (
              <li key={s.browser} className="flex flex-col gap-0.5 border-b border-line py-3.5 sm:flex-row sm:items-baseline sm:gap-3">
                <span className="font-medium">{s.browser}</span>
                <span aria-hidden="true" className="leader hidden sm:block" />
                <span className="font-mono text-xs text-faint sm:text-right">Queued · coming to {s.store}</span>
              </li>
            ))}
          </ul>
          <h4 className="mt-10 font-semibold">Load it from Snag’s install folder today</h4>
          <ol className="mt-4 grid gap-3 leading-relaxed text-muted">
            {[
              <>
                Open <code className={code}>chrome://extensions</code> or <code className={code}>edge://extensions</code>{" "}
                and turn on Developer mode.
              </>,
              <>
                Click Load unpacked and pick the <code className={code}>browser-extension</code> folder in Snag’s install
                folder.
              </>,
              <>Click the Snag icon, then Connect to Snag, and press Allow in Snag.</>,
            ].map((item, i) => (
              <li key={i} className="flex gap-4">
                <span className="w-5 shrink-0 font-display text-lg leading-snug text-text italic">{i + 1}.</span>
                <span>{item}</span>
              </li>
            ))}
          </ol>
          <p className="mt-6 text-sm leading-relaxed text-faint">
            Brave and other Chromium browsers work the same way. Until the Firefox listing is live, you can build the
            extension from the source on GitHub and load it from about:debugging as a temporary add-on.
          </p>
        </div>
        <Plate fig={6} caption="the extension’s popup, connected to Snag." frameClassName="aspect-[16/15]" className="self-start">
          <Shot
            name="ext-popup"
            widths={[720, 1280]}
            height={800}
            sizes="(min-width: 1024px) 900px, 170vw"
            alt="The extension’s popup, connected to Snag, with switches for catching downloads and the media found on the page"
            className="absolute top-0 right-0 w-[150%] max-w-none"
          />
        </Plate>
      </div>
    </Section>
  );
}
