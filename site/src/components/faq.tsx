import { Plus } from "@phosphor-icons/react/dist/ssr";

import { Section, SectionHead } from "@/components/catalogue";
import { ExternalLink } from "@/components/external-link";
import { pad } from "@/lib/parts";
import { RELEASES_URL, REPO_URL } from "@/lib/site";

const link = "text-text underline decoration-accent underline-offset-4 hover:text-accent-text";

const questions: { q: string; value: string; a: React.ReactNode }[] = [
  {
    q: "Is it free?",
    value: "Yes · MIT",
    a: (
      <>
        Yes, completely. Snag is open source under the MIT licence: no trial, no ads, no paid tier. The code is on{" "}
        <ExternalLink href={REPO_URL} className={link}>
          GitHub
        </ExternalLink>
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
          <ExternalLink href={RELEASES_URL} className={link}>
            release on GitHub
          </ExternalLink>{" "}
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
    value: "Yes · both",
    a: "Yes. On a Mac, open Snag-macOS.dmg and drag Snag to Applications (one app for Apple silicon and Intel, macOS 11 or newer). It isn’t signed by Apple yet, so the first time, right-click Snag and choose Open — on macOS 15, open System Settings → Privacy & Security and click Open Anyway. On Linux, download the AppImage (any distro: make it executable and run it) or the .deb for Ubuntu and Debian 22.04 or newer. Snag updates itself on all three.",
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

// Set like an index: the question, a dotted leader, the short answer; open a row for the rest.
export function Faq() {
  return (
    <Section id="faq" className="mx-auto max-w-6xl px-4 pt-28 sm:px-6 md:pt-40">
      <SectionHead
        id="faq"
        title={
          <>
            Fair <em>questions</em>.
          </>
        }
      />
      <div className="mt-14 border-b border-line md:mt-20" data-reveal>
        {questions.map(({ q, value, a }, i) => (
          <details key={q} className="group border-t border-line">
            <summary className="flex cursor-pointer items-baseline gap-4 py-6 md:py-7">
              <span className="w-8 shrink-0 font-mono text-xs text-faint">Q{pad(i + 1)}</span>
              <span className="text-lg font-medium tracking-tight transition-colors group-hover:text-accent-text md:text-2xl">
                {q}
              </span>
              <span aria-hidden="true" className="leader hidden sm:block" />
              <span className="hidden shrink-0 font-mono text-sm text-muted sm:inline">{value}</span>
              <Plus
                weight="light"
                className="ml-auto size-5 shrink-0 self-center text-faint transition-transform duration-300 group-open:rotate-45 sm:ml-0"
                aria-hidden
              />
            </summary>
            <div className="max-w-[64ch] pb-8 pl-12 leading-relaxed text-muted md:text-lg">{a}</div>
          </details>
        ))}
      </div>
    </Section>
  );
}
