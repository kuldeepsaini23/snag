import Link from "next/link";

import { PrivacyDiagram } from "@/components/privacy-diagram";
import { Section, SectionIntro } from "@/components/section";

const points = [
  "No account, no analytics, no server of our own.",
  "Your downloads, settings and history stay in %APPDATA%\\Snag on your PC.",
  "The extension talks only to Snag on your computer (127.0.0.1), and only after you allow it.",
  "Cookies go along only for the site you download from, and only when you send a download.",
  "Clipboard watch, notifications and phone sharing are yours to switch on or off.",
];

export function PrivacySection() {
  return (
    <Section id="privacy" label="02 / Privacy" note="No account · No analytics · No server">
      <SectionIntro
        title={
          <>
            Your files stay <em>yours.</em>
          </>
        }
      >
        Snag runs entirely on your PC. The only line out goes to the sites you download from.
      </SectionIntro>

      <div className="grid gap-px border-t border-line bg-line lg:grid-cols-2">
        <div className="bg-bg px-5 py-8 sm:px-8" data-reveal>
          <ul className="grid">
            {points.map((point, i) => (
              <li key={point} className="flex gap-4 border-b border-line py-4 first:pt-0 last:border-0 last:pb-0">
                <span className="label pt-1 text-faint">{String(i + 1).padStart(2, "0")}</span>
                <span className="leading-relaxed text-muted">{point}</span>
              </li>
            ))}
          </ul>
          <p className="mt-8">
            <Link
              href="/privacy/"
              className="label inline-flex h-10 items-center border border-line-strong px-4 text-text transition-colors hover:border-text"
            >
              Read the privacy policy
            </Link>
          </p>
        </div>
        <div className="bg-bg" data-reveal style={{ "--reveal-delay": "0.08s" } as React.CSSProperties}>
          <PrivacyDiagram />
        </div>
      </div>

      <figure className="border-t border-line" data-reveal>
        <picture>
          <source
            type="image/webp"
            srcSet="/images/permissions-760.webp 760w, /images/permissions-1382.webp 1382w"
            sizes="(min-width: 1056px) 1056px, 100vw"
          />
          <img
            src="/images/permissions-1382.webp"
            alt="Snag's first-run tour: switches for copied links, notifications and phone sharing, and what Snag does with cookies, the internet and the safety check"
            width={1382}
            height={864}
            loading="lazy"
            decoding="async"
            className="block h-auto w-full"
          />
        </picture>
        <figcaption className="label border-t border-line px-5 py-4 leading-relaxed text-faint sm:px-8">
          First run: Snag asks what it may use. Change any of it later in Settings.
        </figcaption>
      </figure>
    </Section>
  );
}
