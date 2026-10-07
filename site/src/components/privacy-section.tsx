import Link from "next/link";
import { Check } from "@phosphor-icons/react/dist/ssr";

import { PrivacyDiagram } from "@/components/privacy-diagram";

const points = [
  "No account, no analytics, no server of our own.",
  "Your downloads, settings and history stay in %APPDATA%\Snag on your PC.",
  "The extension talks only to Snag on your computer (127.0.0.1), and only after you allow it.",
  "Cookies go along only for the site you download from, and only when you send a download.",
  "Clipboard watch, notifications and phone sharing are yours to switch on or off.",
];

export function PrivacySection() {
  return (
    <section id="privacy" className="mx-auto max-w-6xl px-4 pt-28 sm:px-6 md:pt-36">
      <div className="grid gap-12 lg:grid-cols-[1fr_1.1fr] lg:items-center">
        <div>
          <h2 className="heading max-w-[14ch] text-4xl md:text-6xl">Everything stays on your PC</h2>
          <ul className="mt-8 grid gap-4">
            {points.map((point) => (
              <li key={point} className="flex gap-3 leading-relaxed">
                <Check weight="bold" className="mt-1 size-4.5 shrink-0 text-accent" aria-hidden />
                <span className="text-muted">{point}</span>
              </li>
            ))}
          </ul>
          <p className="mt-8">
            <Link href="/privacy/" className="font-semibold text-accent underline decoration-accent/40 underline-offset-4 hover:decoration-accent">
              Read the privacy policy
            </Link>
          </p>
        </div>
        <PrivacyDiagram />
      </div>

      <figure className="mt-14 md:mt-20">
        <picture>
          <source
            type="image/webp"
            srcSet="/images/permissions-760.webp 760w, /images/permissions-1382.webp 1382w"
            sizes="(min-width: 1152px) 1104px, calc(100vw - 32px)"
          />
          <img
            src="/images/permissions-1382.webp"
            alt="Snag's first-run tour: switches for copied links, notifications and phone sharing, and what Snag does with cookies, the internet and the safety check"
            width={1382}
            height={864}
            loading="lazy"
            decoding="async"
            className="block h-auto w-full rounded-[14px] ring-1 ring-line-strong"
          />
        </picture>
        <figcaption className="mt-4 max-w-[60ch] text-sm text-muted">
          The first time you open Snag, it asks what it may use. You can change any of it later in Settings.
        </figcaption>
      </figure>
    </section>
  );
}
