import { CursorClick, DownloadSimple, Plug } from "@phosphor-icons/react/dist/ssr";

import { INSTALLER_NAME } from "@/lib/site";

const steps = [
  {
    icon: DownloadSimple,
    title: "Install Snag",
    body: "Run the installer. It sets Snag up for your Windows user only, so it needs no admin rights.",
    hint: `${INSTALLER_NAME} · 12 MB`,
  },
  {
    icon: Plug,
    title: "Connect the browser extension",
    body: "Click the Snag icon in your browser, then Connect to Snag, and press Allow in Snag. You do this once.",
    hint: "One click, then Allow",
  },
  {
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

const code = "rounded bg-raised px-1.5 py-0.5 font-mono text-[0.85em] text-text";

export function HowItWorks() {
  return (
    <section id="how-it-works" className="mx-auto max-w-6xl px-4 pt-24 sm:px-6 md:pt-32">
      <h2 className="text-4xl font-bold tracking-tight md:text-5xl">Set up in a minute</h2>

      {/* Three cards in the style of Snag's first-run tour. */}
      <ol className="mt-10 grid gap-4 md:grid-cols-3">
        {steps.map((step, i) => {
          const Icon = step.icon;
          return (
            <li key={step.title} className="flex flex-col rounded-2xl bg-panel p-5 shadow-[0_0_0_1px_var(--line)] sm:p-6">
              <div className="flex items-center justify-between">
                <span className="flex items-center gap-1" aria-hidden="true">
                  {steps.map((_, j) => (
                    <span
                      key={j}
                      className={j === i ? "h-1.5 w-4 rounded-full bg-accent" : "size-1.5 rounded-full bg-accent/40"}
                    />
                  ))}
                </span>
                <span className="text-xs text-faint">
                  Step {i + 1} of {steps.length}
                </span>
              </div>
              <span className="mt-6 grid size-12 place-items-center rounded-xl bg-accent/15">
                <Icon className="size-6 text-accent" aria-hidden />
              </span>
              <h3 className="mt-4 text-lg font-semibold">{step.title}</h3>
              <p className="mt-1.5 leading-relaxed text-muted">{step.body}</p>
              <p className="mt-auto pt-6 font-mono text-xs text-faint">{step.hint}</p>
            </li>
          );
        })}
      </ol>

      <div id="extension" className="mt-4 grid gap-4 rounded-2xl bg-panel p-2 shadow-[0_0_0_1px_var(--line)] sm:p-3 lg:grid-cols-2">
        <div className="p-3 sm:p-4">
          <h3 className="text-lg font-semibold">The browser extension</h3>
          <ul className="mt-4 divide-y divide-line rounded-xl bg-raised/60">
            {stores.map((s) => (
              <li key={s.browser} className="flex items-center justify-between gap-4 px-4 py-3">
                <span className="font-medium">{s.browser}</span>
                <span className="font-mono text-xs text-faint">Queued · coming to {s.store}</span>
              </li>
            ))}
          </ul>
          <h4 className="mt-6 font-semibold">Load it from Snag’s install folder today</h4>
          <ol className="mt-3 grid list-decimal gap-2 pl-5 leading-relaxed text-muted marker:font-mono marker:text-sm marker:text-faint">
            <li>
              Open <code className={code}>chrome://extensions</code> or <code className={code}>edge://extensions</code> and
              turn on Developer mode.
            </li>
            <li>
              Click Load unpacked and pick the <code className={code}>browser-extension</code> folder in Snag’s install
              folder.
            </li>
            <li>Click the Snag icon, then Connect to Snag, and press Allow in Snag.</li>
          </ol>
          <p className="mt-4 text-sm leading-relaxed text-faint">
            Brave and other Chromium browsers work the same way. Until the Firefox listing is live, you can build the
            extension from the source on GitHub and load it from about:debugging as a temporary add-on.
          </p>
        </div>
        <figure className="relative min-h-72 overflow-hidden rounded-xl bg-bg">
          <picture>
            <source
              type="image/webp"
              srcSet="/images/ext-popup-720.webp 720w, /images/ext-popup-1280.webp 1280w"
              sizes="(min-width: 1024px) 840px, 160vw"
            />
            <img
              src="/images/ext-popup-1280.webp"
              alt="The extension's popup, connected to Snag, with switches for catching downloads and the media found on the page"
              width={1280}
              height={800}
              loading="lazy"
              decoding="async"
              className="absolute top-0 right-0 h-auto w-[160%] max-w-none"
            />
          </picture>
        </figure>
      </div>
    </section>
  );
}
