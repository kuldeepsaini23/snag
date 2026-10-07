import { GithubLogo, WindowsLogo } from "@phosphor-icons/react/dist/ssr";

import { ButtonLink } from "@/components/button";
import { SegmentedWord } from "@/components/segments";
import { SpeedLine } from "@/components/speed-line";
import { DOWNLOAD_URL, PITCH, REPO_URL } from "@/lib/site";

export function Hero() {
  return (
    <section className="relative overflow-hidden">
      <div className="relative mx-auto max-w-6xl px-4 pt-12 sm:px-6 md:pt-20">
        <div className="grid gap-x-12 gap-y-8 lg:grid-cols-[auto_1fr] lg:items-end">
          <h1 className="text-[clamp(6rem,38vw,11rem)] leading-[0.85] font-extrabold tracking-[-0.055em] lg:text-[13rem]">
            <SegmentedWord text="Snag" />
          </h1>
          <div className="max-w-xl lg:pb-4">
            <p className="text-xl leading-snug font-medium text-balance md:text-2xl">{PITCH}</p>
            <div className="mt-7 flex flex-col gap-3 sm:flex-row">
              <ButtonLink href={DOWNLOAD_URL}>
                <WindowsLogo weight="fill" className="size-5" aria-hidden />
                Download for Windows
              </ButtonLink>
              <ButtonLink href={REPO_URL} variant="secondary">
                <GithubLogo weight="fill" className="size-5" aria-hidden />
                View on GitHub
              </ButtonLink>
            </div>
            <p className="mt-4 font-mono text-xs text-faint">Windows 10 &amp; 11 · 12 MB · Mac &amp; Linux soon</p>
          </div>
        </div>
      </div>

      {/* The app in its own window, with its speed graph running behind it. On phones the window runs
          off the right edge rather than shrinking past legibility. */}
      <div className="relative mt-14 md:mt-20">
        <SpeedLine id="hero" className="absolute inset-x-0 -top-12 h-28 opacity-50 md:-top-28 md:h-52" />
        <div className="relative mx-auto max-w-6xl pl-4 sm:px-6">
          <figure className="w-[720px] max-w-none overflow-hidden rounded-xl bg-panel shadow-[0_40px_90px_-30px_rgba(0,0,0,0.85),0_0_0_1px_var(--line-strong)] sm:w-full">
            <picture>
              <source
                type="image/webp"
                srcSet="/images/main-760.webp 760w, /images/main-1382.webp 1382w"
                sizes="(min-width: 1152px) 1104px, (min-width: 640px) calc(100vw - 48px), 720px"
              />
              <img
                src="/images/main-1382.webp"
                alt="Snag's main window: three downloads in progress, one at 8.4 MB/s, with its segment map and speed graph"
                width={1382}
                height={864}
                fetchPriority="high"
                className="block h-auto w-full"
              />
            </picture>
          </figure>
        </div>
      </div>
    </section>
  );
}
