const steps = [
  {
    title: "Install Snag",
    body: "Run the installer. It sets Snag up for your Windows user only, so it needs no admin rights.",
  },
  {
    title: "Connect the browser extension",
    body: "Click the Snag icon in your browser, then Connect to Snag, and press Allow in Snag. You do this once.",
  },
  {
    title: "Click Download on any video",
    body: "A Download button appears on videos as they play. Pick a quality and Snag takes it from there.",
  },
];

export function HowItWorks() {
  return (
    <section id="how-it-works" className="mx-auto max-w-6xl px-4 pt-28 sm:px-6 md:pt-36">
      <h2 className="heading text-4xl md:text-6xl">Set up in a minute</h2>
      <ol className="mt-12 grid gap-10 md:grid-cols-3 md:gap-8">
        {steps.map((step, i) => (
          <li key={step.title} className="relative">
            <div className="flex items-center gap-4">
              <span className="display text-6xl text-accent tabular-nums" aria-hidden="true">
                {i + 1}
              </span>
              <span aria-hidden="true" className="h-px flex-1 bg-line-strong" />
            </div>
            <h3 className="heading mt-5 text-xl md:text-2xl">{step.title}</h3>
            <p className="mt-2 max-w-[38ch] leading-relaxed text-muted">{step.body}</p>
          </li>
        ))}
      </ol>
    </section>
  );
}
