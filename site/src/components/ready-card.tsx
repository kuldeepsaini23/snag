import { CheckCircle, GithubLogo, WindowsLogo } from "@phosphor-icons/react/dist/ssr";

import { ButtonLink } from "@/components/button";
import { LogoMark } from "@/components/logo";
import { SpeedLine } from "@/components/speed-line";
import { DOWNLOAD_URL, REPO_URL } from "@/lib/site";

// The end of the page: the download has finished and is ready to open.
export function ReadyCard() {
  return (
    <section id="get-started" className="mx-auto max-w-6xl px-4 py-24 sm:px-6 md:py-32">
      <div className="relative overflow-hidden rounded-3xl bg-panel shadow-[0_0_0_1px_var(--line),0_40px_90px_-40px_rgba(0,0,0,0.8)]">
        <SpeedLine id="ready" duration={24} className="absolute inset-x-0 bottom-0 h-20 opacity-40" />
        <div className="relative px-5 pt-12 pb-24 text-center sm:px-10 md:pt-16 md:pb-28">
          <p className="inline-flex items-center gap-2 rounded-full bg-raised px-3 py-1 font-mono text-xs text-accent">
            <CheckCircle weight="fill" className="size-4" aria-hidden />
            Download complete
          </p>
          <h2 className="mt-6 text-4xl font-bold tracking-tight md:text-6xl">Your download is ready</h2>

          <div className="mx-auto mt-10 flex max-w-md items-center gap-4 rounded-xl bg-raised p-3 text-left">
            <LogoMark size={44} className="size-11 shrink-0" />
            <div className="min-w-0 flex-1">
              <p className="truncate font-medium">Snag-Setup-1.0.0.exe</p>
              <p className="font-mono text-xs text-faint">12 MB · Windows 10 &amp; 11</p>
              <div className="mt-2 flex h-1 gap-[3px]" aria-hidden="true">
                {Array.from({ length: 8 }, (_, i) => (
                  <span key={i} className="flex-1 rounded-full bg-accent" />
                ))}
              </div>
            </div>
          </div>

          <div className="mt-8 flex flex-col justify-center gap-3 sm:flex-row">
            <ButtonLink href={DOWNLOAD_URL}>
              <WindowsLogo weight="fill" className="size-5" aria-hidden />
              Download for Windows
            </ButtonLink>
            <ButtonLink href={REPO_URL} variant="secondary">
              <GithubLogo weight="fill" className="size-5" aria-hidden />
              View on GitHub
            </ButtonLink>
          </div>
          <p className="mt-5 text-sm text-muted">Free and open source. No account. Mac &amp; Linux soon.</p>
        </div>
      </div>
    </section>
  );
}
