import { CaretDown } from "@phosphor-icons/react/dist/ssr";

import { RELEASES_URL, REPO_URL } from "@/lib/site";

const questions: { q: string; value: string; a: React.ReactNode }[] = [
  {
    q: "Is it free?",
    value: "Yes · MIT",
    a: (
      <>
        Yes, completely. Snag is open source under the MIT licence: no trial, no ads, no paid tier. The code is on{" "}
        <a href={REPO_URL} className="text-accent underline decoration-accent/40 underline-offset-4 hover:decoration-accent">
          GitHub
        </a>
        .
      </>
    ),
  },
  {
    q: "Is it safe?",
    value: "Open source",
    a: (
      <>
        <p>
          The whole app is open source, so anyone can read what it does. Each{" "}
          <a href={RELEASES_URL} className="text-accent underline decoration-accent/40 underline-offset-4 hover:decoration-accent">
            release on GitHub
          </a>{" "}
          lists the installer’s SHA-256 checksum, so you can check your file is the one that was published.
        </p>
        <p className="mt-3">
          The installer isn’t code-signed yet, so Windows may show “Windows protected your PC” the first time. Click{" "}
          <strong className="font-semibold text-text">More info</strong>, then{" "}
          <strong className="font-semibold text-text">Run anyway</strong>.
        </p>
      </>
    ),
  },
  {
    q: "Is there a Mac or Linux version?",
    value: "Coming",
    a: "Not yet. Both are coming. For now Snag runs on Windows 10 and 11.",
  },
  {
    q: "Can it download from Netflix or Spotify?",
    value: "No · DRM",
    a: "No. Those services protect their streams with DRM, and Snag does not and will not break DRM. Please download only what you have the right to, and respect the work of the people who made it.",
  },
  {
    q: "Why does it need ffmpeg?",
    value: "Once · 35 MB",
    a: "To join HD video with its sound and to make MP3s. Snag fetches it once, the first time you need it (about 35 MB, checked against its published checksum). Ordinary file downloads never need it.",
  },
];

// Styled after the app's details panel: a label on the left, its value on the right; open a row for the rest.
export function Faq() {
  return (
    <section id="faq" className="mx-auto max-w-6xl px-4 pt-24 sm:px-6 md:pt-32">
      <h2 className="text-4xl font-bold tracking-tight md:text-5xl">Questions</h2>
      <div className="mt-10 max-w-3xl divide-y divide-line overflow-hidden rounded-2xl bg-panel shadow-[0_0_0_1px_var(--line)]">
        {questions.map(({ q, value, a }) => (
          <details key={q} className="group">
            <summary className="flex cursor-pointer items-center gap-4 px-5 py-4 transition-colors hover:bg-raised/50 group-open:bg-raised/50">
              <span className="font-medium">{q}</span>
              <span className="ml-auto hidden shrink-0 font-mono text-sm text-muted sm:inline">{value}</span>
              <CaretDown
                weight="bold"
                className="size-4 shrink-0 text-faint transition-transform duration-200 group-open:rotate-180"
                aria-hidden
              />
            </summary>
            <div className="max-w-[64ch] px-5 pt-1 pb-5 leading-relaxed text-muted">{a}</div>
          </details>
        ))}
      </div>
    </section>
  );
}
