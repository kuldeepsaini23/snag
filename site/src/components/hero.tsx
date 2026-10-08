import { GithubLogo, WindowsLogo } from "@phosphor-icons/react/dist/ssr";

import { ButtonLink } from "@/components/button";
import { Plate, Section, Shot } from "@/components/catalogue";
import { DOWNLOAD_URL, REPO_URL, VERSION } from "@/lib/site";

// The frontispiece: the engraved hook hangs from the download bar beside the headline, and the
// app itself is the first plate.
export function Hero() {
  return (
    <Section id="top" className="overflow-hidden">
      <div className="relative mx-auto max-w-6xl px-4 sm:px-6">
        {/* The engraving, on its guilloche. It hangs from the very top, so its line meets the bar. */}
        <div
          aria-hidden="true"
          className="pointer-events-none absolute top-0 right-3 h-[13.5rem] w-[7rem] sm:right-8 sm:h-[17rem] sm:w-[9rem] lg:right-12 lg:h-[37rem] lg:w-[22rem]"
        >
          <div className="rosette absolute top-[38%] left-1/2 size-[17rem] -translate-x-1/2 -translate-y-1/2 sm:size-[22rem] lg:size-[38rem]" />
          <div className="rosette absolute top-[38%] left-1/2 size-[11rem] -translate-x-1/2 -translate-y-1/2 opacity-60 [background:var(--accent)] sm:size-[14rem] lg:size-[24rem]" />
          {/* eslint-disable-next-line @next/next/no-img-element -- static export, pre-sized WebP */}
          <img
            src="/images/hook-560.webp"
            srcSet="/images/hook-560.webp 196w, /images/hook-960.webp 337w"
            sizes="(min-width: 1024px) 13rem, (min-width: 640px) 6rem, 4.75rem"
            alt=""
            width={196}
            height={560}
            className="hook relative mx-auto h-full w-auto"
          />
        </div>

        <div className="relative pt-8 sm:pt-12 lg:pt-24 lg:pr-[24rem]">
          <p className="label">
            {VERSION ? `Vol. ${VERSION}` : "Snag"} · For Windows
          </p>
          <p className="label mt-1.5 max-w-[11rem] normal-case tracking-normal sm:max-w-none">
            <span className="font-display text-sm italic">Pl. I — the hook and the arrow</span>
          </p>
          <h1 className="headline mt-24 text-[clamp(3.4rem,16vw,9.25rem)] sm:mt-32 lg:mt-14">
            Download <em>anything</em>.
          </h1>
          <p className="mt-8 max-w-[36ch] text-xl leading-snug text-muted md:text-2xl">
            <span className="text-text">The fast, free download manager for Windows</span> — catches any video like
            IDM, downloads files 8× in parallel
          </p>
          <div className="mt-9 flex flex-col gap-3 sm:flex-row">
            <ButtonLink href={DOWNLOAD_URL}>
              <WindowsLogo weight="fill" className="size-5" aria-hidden />
              Download for Windows
            </ButtonLink>
            <ButtonLink href={REPO_URL} variant="secondary">
              <GithubLogo weight="fill" className="size-5" aria-hidden />
              View on GitHub
            </ButtonLink>
          </div>
          <p className="mt-5 font-mono text-xs text-faint">Windows 10 &amp; 11 · 12 MB · Mac &amp; Linux soon</p>
        </div>
      </div>

      <div className="mx-auto mt-16 max-w-6xl px-4 sm:px-6 md:mt-24">
        <Plate
          fig={1}
          caption="Snag’s main window: three downloads under way, one at 8.4 MB/s."
          frameClassName="aspect-[4/3] sm:aspect-auto"
        >
          {/* On phones the plate shows the left of the window at a readable size. */}
          <Shot
            name="app"
            widths={[720, 1280]}
            height={800}
            sizes="(min-width: 1152px) 1090px, (min-width: 640px) calc(100vw - 60px), 175vw"
            alt="Snag’s main window: a library of downloads by type, three downloads in progress with one at 8.4 MB/s, and the details of the selected video"
            className="absolute top-0 left-0 w-[175%] max-w-none sm:static sm:w-full"
          />
        </Plate>
      </div>
    </Section>
  );
}
