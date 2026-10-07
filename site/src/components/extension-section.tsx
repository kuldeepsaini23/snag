const browsers = [
  { name: "Chrome", store: "the Chrome Web Store" },
  { name: "Edge", store: "Edge Add-ons" },
  { name: "Firefox", store: "Firefox Add-ons" },
];

export function ExtensionSection() {
  return (
    <section id="extension" className="mx-auto max-w-6xl px-4 pt-28 sm:px-6 md:pt-36">
      <div className="rounded-[28px] border border-line bg-surface p-6 md:p-12">
        <h2 className="heading max-w-[20ch] text-3xl md:text-5xl">The browser extension is coming to the stores</h2>
        <ul className="mt-8 grid gap-3 sm:grid-cols-3">
          {browsers.map((browser) => (
            <li key={browser.name} className="rounded-2xl border border-line bg-raised p-5">
              <p className="text-lg font-semibold">{browser.name}</p>
              <p className="mt-1 text-sm text-muted">Coming to {browser.store}</p>
            </li>
          ))}
        </ul>

        <div className="mt-10 grid gap-8 md:grid-cols-2">
          <div>
            <h3 className="heading text-xl">Load it from Snag’s install folder today</h3>
            <ol className="mt-4 grid list-decimal gap-2 pl-5 leading-relaxed text-muted marker:font-semibold marker:text-accent">
              <li>
                Open <code className="rounded bg-raised px-1.5 py-0.5 text-[0.9em] text-text">chrome://extensions</code> (or{" "}
                <code className="rounded bg-raised px-1.5 py-0.5 text-[0.9em] text-text">edge://extensions</code>) and turn
                on Developer mode.
              </li>
              <li>
                Click Load unpacked and pick the{" "}
                <code className="rounded bg-raised px-1.5 py-0.5 text-[0.9em] text-text">browser-extension</code> folder in
                Snag’s install folder.
              </li>
              <li>Click the Snag icon, then Connect to Snag, and press Allow in Snag.</li>
            </ol>
            <p className="mt-4 text-sm text-faint">Brave and other Chromium browsers work the same way.</p>
          </div>
          <div>
            <h3 className="heading text-xl">Firefox</h3>
            <p className="mt-4 max-w-[46ch] leading-relaxed text-muted">
              Until the Firefox Add-ons listing is live, you can build the extension from the source on GitHub and load it
              from about:debugging as a temporary add-on.
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}
