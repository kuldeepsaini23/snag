import { GithubLogo, WindowsLogo } from "@phosphor-icons/react/dist/ssr";

import { ButtonLink } from "@/components/button";
import { LogoMark } from "@/components/logo";
import { SegmentBar } from "@/components/segment-bar";
import { DOWNLOAD_URL, PITCH, REPO_URL } from "@/lib/site";

function intro(delay: number) {
  return { "--intro-delay": `${delay}s` } as React.CSSProperties;
}

export function Hero() {
  return (
    <section id="top" aria-label="Snag">
      <div className="flex min-h-10 items-center justify-between gap-4 border-b border-line px-5 py-3 sm:px-8">
        <p className="label text-muted">Snag 1.0 / Windows</p>
        <p className="label hidden text-faint sm:block">Free · Open source · MIT</p>
      </div>

      <div className="relative overflow-hidden px-5 pt-10 pb-12 sm:px-8 md:pt-14 md:pb-16">
        <div
          aria-hidden="true"
          className="pointer-events-none absolute top-0 left-1/2 h-72 w-[40rem] -translate-x-1/2 bg-[radial-gradient(closest-side,color-mix(in_oklab,var(--accent)_14%,transparent),transparent)]"
        />
        <div className="relative flex justify-center animate-intro">
          <LogoMark size={72} className="size-16 md:size-[72px]" />
        </div>

        <h1
          className="display relative mt-10 text-[clamp(3.1rem,12.5vw,7.25rem)] animate-intro md:mt-12"
          style={intro(0.3)}
        >
          Download <em>anything.</em>
        </h1>
        <SegmentBar className="mt-7 h-1.5 max-w-xl md:mt-9" />

        <div className="relative animate-intro" style={intro(0.5)}>
          <p className="mt-8 max-w-[46ch] text-lg leading-relaxed text-muted md:text-xl">{PITCH}</p>
          <div className="mt-9 flex flex-col gap-3 sm:flex-row">
            <ButtonLink href={DOWNLOAD_URL}>
              <WindowsLogo weight="fill" className="size-4" aria-hidden />
              Download for Windows
            </ButtonLink>
            <ButtonLink href={REPO_URL} variant="outline">
              <GithubLogo weight="fill" className="size-4" aria-hidden />
              View on GitHub
            </ButtonLink>
          </div>
          <p className="label mt-6 leading-relaxed text-faint">Windows 10 &amp; 11 · 12 MB · Mac &amp; Linux coming soon</p>
        </div>
      </div>

      {/* Full column width between hairlines; on phones it runs off the right edge rather than shrinking. */}
      <figure className="overflow-hidden border-t border-line bg-sunk">
        <picture>
          <source
            type="image/webp"
            srcSet="/images/main-760.webp 760w, /images/main-1382.webp 1382w"
            sizes="(min-width: 1056px) 1056px, (min-width: 640px) 100vw, 720px"
          />
          <img
            src="/images/main-1382.webp"
            alt="Snag's main window: three downloads in progress, one at 8.4 MB/s, with its segment map and speed graph"
            width={1382}
            height={864}
            fetchPriority="high"
            className="block h-auto w-[720px] max-w-none sm:w-full"
          />
        </picture>
      </figure>
    </section>
  );
}
