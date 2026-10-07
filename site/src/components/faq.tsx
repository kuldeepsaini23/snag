import { Plus } from "@phosphor-icons/react/dist/ssr";

import { Section, SectionIntro } from "@/components/section";
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
    <Section id="faq" label="04 / FAQ" note="Still wondering?">
      <SectionIntro
        title={
          <>
            Questions, <em>answered.</em>
          </>
        }
      />
      <div className="border-t border-line">
        {questions.map(({ q, a }, i) => (
          <details key={q} className="group border-b border-line last:border-0" data-reveal>
            <summary className="flex cursor-pointer items-center gap-5 px-5 py-5 transition-colors hover:bg-white/[0.02] sm:px-8">
              <span className="label w-6 shrink-0 text-faint">{String(i + 1).padStart(2, "0")}</span>
              <span className="font-display text-lg leading-snug font-bold [font-stretch:110%]">{q}</span>
              <span className="ml-auto grid size-8 shrink-0 place-items-center border border-line-strong text-muted">
                <Plus weight="bold" className="size-3.5 transition-transform duration-200 group-open:rotate-45" aria-hidden />
              </span>
            </summary>
            <div className="max-w-[64ch] px-5 pb-7 pl-[3.25rem] leading-relaxed text-muted sm:px-8 sm:pl-[4.75rem]">{a}</div>
          </details>
        ))}
      </div>
    </Section>
  );
}
