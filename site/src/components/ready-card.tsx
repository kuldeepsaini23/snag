import { CheckCircle, GithubLogo } from "@phosphor-icons/react/dist/ssr";

import { ButtonLink } from "@/components/button";
import { Band, Section } from "@/components/catalogue";
import { DownloadMenu } from "@/components/download-menu";
import { LogoMark } from "@/components/logo";
import { pad, parts } from "@/lib/parts";
import { DOWNLOAD_URL, INSTALLER_NAME, RELEASES_URL, REPO_URL, VERSION } from "@/lib/site";

function Corner({ className }: { className: string }) {
  return <span aria-hidden="true" className={`rosette absolute size-16 [background:var(--line-strong)] md:size-24 ${className}`} />;
}

// The end of the page: the download is finished, issued like a banknote, guilloche and all.
export function ReadyCard() {
  return (
    <Section id="get-started" className="mx-auto max-w-6xl px-4 pt-28 pb-24 sm:px-6 md:pt-40 md:pb-32">
      <div className="border border-line-strong p-1.5" data-reveal>
        <div className="relative overflow-hidden border border-line bg-surface">
          <Band />
          <div aria-hidden="true" className="rosette absolute top-1/2 left-1/2 size-[46rem] -translate-1/2" />
          <div
            aria-hidden="true"
            className="rosette absolute top-1/2 left-1/2 size-[28rem] -translate-1/2 opacity-30 [background:var(--accent)]"
          />
          <Corner className="top-12 left-3 md:top-14 md:left-5" />
          <Corner className="top-12 right-3 md:top-14 md:right-5" />
          <Corner className="right-3 bottom-12 md:right-5 md:bottom-14" />
          <Corner className="bottom-12 left-3 md:bottom-14 md:left-5" />

          <div className="relative px-5 py-16 text-center sm:px-10 md:py-24">
            <p className="label inline-flex items-center gap-2 text-accent-text">
              <CheckCircle weight="fill" className="size-4" aria-hidden />
              Part {pad(parts.length)} of {pad(parts.length)} · Download complete
            </p>
            <h2 className="headline mx-auto mt-8 max-w-[12ch] text-[clamp(2.75rem,10vw,6rem)]">
              Your download is <em>ready</em>.
            </h2>

            <div className="mx-auto mt-12 flex max-w-md items-center gap-4 border border-line-strong bg-bg p-3 text-left">
              <LogoMark size={44} className="size-11 shrink-0" />
              <div className="min-w-0 flex-1">
                <p className="truncate font-medium">{INSTALLER_NAME}</p>
                <p className="font-mono text-xs text-faint">12 MB · Windows 10 &amp; 11</p>
                <div className="mt-2 flex h-1 gap-[3px]" aria-hidden="true">
                  {parts.map((part) => (
                    <span key={part.id} className="flex-1 bg-accent" />
                  ))}
                </div>
              </div>
            </div>

            <div className="mt-9 flex flex-col justify-center gap-3 sm:flex-row">
              <DownloadMenu downloadUrl={DOWNLOAD_URL} releasesUrl={RELEASES_URL} version={VERSION} menuAlign="center" />
              <ButtonLink href={REPO_URL} variant="secondary" className="bg-bg">
                <GithubLogo weight="fill" className="size-5" aria-hidden />
                View on GitHub
              </ButtonLink>
            </div>
            <p className="mt-6 font-display text-lg text-muted italic">Free and open source. No account. Mac &amp; Linux soon.</p>
          </div>
          <Band />
        </div>
      </div>
    </Section>
  );
}
