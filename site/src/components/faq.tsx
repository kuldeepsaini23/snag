import { Plus } from "@phosphor-icons/react/dist/ssr";

import { RELEASES_URL, REPO_URL } from "@/lib/site";

const questions: { q: string; a: React.ReactNode }[] = [
  {
    q: "Is it free?",
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
    a: "Not yet. Both are coming. For now Snag runs on Windows 10 and 11.",
  },
  {
    q: "Can it download from Netflix or Spotify?",
    a: "No. Those services protect their streams with DRM, and Snag does not and will not break DRM. Please download only what you have the right to, and respect the work of the people who made it.",
  },
  {
    q: "Why does it need ffmpeg?",
    a: "To join HD video with its sound and to make MP3s. Snag fetches it once, the first time you need it (about 35 MB, checked against its published checksum). Ordinary file downloads never need it.",
  },
];

export function Faq() {
  return (
    <section id="faq" className="mx-auto max-w-6xl px-4 pt-28 sm:px-6 md:pt-36">
      <div className="grid gap-10 lg:grid-cols-[1fr_1.6fr]">
        <h2 className="heading text-4xl md:text-6xl">Questions</h2>
        <div className="border-t border-line">
          {questions.map(({ q, a }) => (
            <details key={q} className="group border-b border-line">
              <summary className="flex cursor-pointer items-center gap-4 py-5 text-lg font-semibold transition-colors hover:text-accent">
                {q}
                <Plus
                  weight="bold"
                  className="ml-auto size-5 shrink-0 text-muted transition-transform duration-200 group-open:rotate-45"
                  aria-hidden
                />
              </summary>
              <div className="max-w-[62ch] pb-6 leading-relaxed text-muted">{a}</div>
            </details>
          ))}
        </div>
      </div>
    </section>
  );
}
