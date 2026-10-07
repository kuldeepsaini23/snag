import { GithubLogo } from "@phosphor-icons/react/dist/ssr";

import { DownloadButton } from "@/components/download-button";
import { SegmentBar } from "@/components/segment-bar";
import { BorderBeam } from "@/components/ui/border-beam";
import { PITCH, REPO_URL } from "@/lib/site";

export function Hero() {
  return (
    <section className="relative overflow-hidden">
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-x-0 top-0 h-[42rem] bg-[radial-gradient(60rem_28rem_at_85%_-10%,color-mix(in_oklab,var(--accent)_16%,transparent),transparent_70%)]"
      />
      <div className="relative mx-auto max-w-6xl px-4 pt-14 sm:px-6 md:pt-24">
        <div className="grid gap-x-12 gap-y-8 lg:grid-cols-[auto_1fr] lg:items-end">
          <div className="w-fit">
            <h1 className="display text-[clamp(5rem,28vw,12.5rem)] lg:text-[10rem] xl:text-[11.5rem]">Snag</h1>
            <SegmentBar className="mt-4 h-3 md:mt-5 md:h-3.5" />
          </div>
          <div className="max-w-xl lg:pb-1">
            <p className="text-[1.4rem] leading-snug font-semibold text-balance [font-stretch:108%] md:text-[1.7rem]">
              {PITCH}
            </p>
            <div className="mt-7 flex flex-col gap-3 sm:flex-row sm:flex-wrap sm:items-center">
              <DownloadButton />
              <a
                href={REPO_URL}
                className="inline-flex h-13 items-center justify-center gap-2 whitespace-nowrap rounded-full border border-line-strong px-6 font-semibold transition-colors hover:border-muted hover:bg-raised"
              >
                <GithubLogo weight="fill" className="size-5" aria-hidden />
                View on GitHub
              </a>
            </div>
            <p className="mt-4 text-sm text-muted">Windows 10 &amp; 11 · 12 MB · Mac &amp; Linux coming soon</p>
          </div>
        </div>
      </div>

      {/* On phones the window runs off the right edge rather than shrinking past legibility. */}
      <div className="relative mx-auto mt-14 max-w-6xl pl-4 sm:px-6 md:mt-20">
        <div
          aria-hidden="true"
          className="pointer-events-none absolute inset-x-[8%] top-[12%] bottom-0 rounded-full bg-accent/25 blur-[90px]"
        />
        <figure className="relative w-[720px] max-w-none rounded-[14px] bg-surface shadow-[0_50px_100px_-30px_rgba(0,0,0,0.8)] ring-1 ring-line-strong sm:w-full">
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
              className="block h-auto w-full rounded-[14px]"
            />
          </picture>
          <BorderBeam size={260} duration={12} colorFrom="var(--accent)" colorTo="#ffd28a" borderWidth={1.5} />
        </figure>
      </div>
    </section>
  );
}
