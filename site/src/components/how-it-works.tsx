import { Section, SectionIntro } from "@/components/section";

const steps = [
  {
    title: "Install Snag",
    body: "Run the installer. It sets Snag up for your Windows user only, so it needs no admin rights.",
  },
  {
    title: "Connect the extension",
    body: "Click the Snag icon in your browser, then Connect to Snag, and press Allow in Snag. You do this once.",
  },
  {
    title: "Click Download on any video",
    body: "A Download button appears on videos as they play. Pick a quality and Snag takes it from there.",
  },
];

const stores = [
  { browser: "Chrome", store: "Chrome Web Store" },
  { browser: "Edge", store: "Edge Add-ons" },
  { browser: "Firefox", store: "Firefox Add-ons" },
];

const code = "bg-raised px-1.5 py-0.5 font-mono text-[0.85em] text-text";

export function HowItWorks() {
  return (
    <Section id="how-it-works" label="03 / How it works" note="About a minute">
      <SectionIntro
        title={
          <>
            Set up in a <em>minute.</em>
          </>
        }
      />

      <ol className="grid gap-px border-t border-line bg-line md:grid-cols-3">
        {steps.map((step, i) => (
          <li
            key={step.title}
            className="bg-bg px-5 pt-6 pb-10 sm:px-8"
            data-reveal
            style={{ "--reveal-delay": `${i * 0.08}s` } as React.CSSProperties}
          >
            <p className="label text-accent">Step {String(i + 1).padStart(2, "0")}</p>
            <h3 className="mt-10 font-display text-xl leading-tight font-bold [font-stretch:115%] md:mt-16">
              {step.title}
            </h3>
            <p className="mt-2 leading-relaxed text-muted">{step.body}</p>
          </li>
        ))}
      </ol>

      <div id="extension" className="border-t border-line">
        <div className="flex min-h-10 items-center justify-between gap-4 border-b border-line px-5 py-3 sm:px-8">
          <p className="label text-muted">Browser extension</p>
          <p className="label text-faint">Coming to the stores</p>
        </div>
        <div className="grid gap-px bg-line lg:grid-cols-2">
          <div className="bg-bg" data-reveal>
            <ul>
              {stores.map((s) => (
                <li
                  key={s.browser}
                  className="flex items-baseline justify-between gap-4 border-b border-line px-5 py-4 sm:px-8"
                >
                  <span className="font-display text-lg font-bold [font-stretch:115%]">{s.browser}</span>
                  <span className="label text-right text-faint">Coming to {s.store}</span>
                </li>
              ))}
            </ul>
            <div className="px-5 py-8 sm:px-8">
              <h3 className="font-display text-lg font-bold [font-stretch:115%]">Load it from Snag’s install folder today</h3>
              <ol className="mt-4 grid list-decimal gap-2 pl-5 leading-relaxed text-muted marker:font-mono marker:text-sm marker:text-faint">
                <li>
                  Open <code className={code}>chrome://extensions</code> or <code className={code}>edge://extensions</code>{" "}
                  and turn on Developer mode.
                </li>
                <li>
                  Click Load unpacked and pick the <code className={code}>browser-extension</code> folder in Snag’s
                  install folder.
                </li>
                <li>Click the Snag icon, then Connect to Snag, and press Allow in Snag.</li>
              </ol>
              <p className="mt-5 text-sm leading-relaxed text-faint">
                Brave and other Chromium browsers work the same way. Until the Firefox listing is live, you can build the
                extension from the source on GitHub and load it from about:debugging as a temporary add-on.
              </p>
            </div>
          </div>
          <figure className="relative min-h-80 overflow-hidden bg-sunk" data-reveal style={{ "--reveal-delay": "0.08s" } as React.CSSProperties}>
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
      </div>
    </Section>
  );
}
