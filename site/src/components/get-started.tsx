import { GithubLogo, WindowsLogo } from "@phosphor-icons/react/dist/ssr";

import { ButtonLink } from "@/components/button";
import { Section } from "@/components/section";
import { DOWNLOAD_URL, REPO_URL } from "@/lib/site";

export function GetStarted() {
  return (
    <Section id="get-started" label="Get started" note="Windows 10 & 11 · 12 MB">
      <div className="flex flex-col items-center px-5 py-24 text-center sm:px-8 md:py-32" data-reveal>
        <p className="label text-faint">Free · Open source · No account</p>
        <h2 className="display mt-6 text-[clamp(3rem,10vw,6.5rem)]">
          Start <em>snagging.</em>
        </h2>
        <div className="mt-10 flex w-full flex-col justify-center gap-3 sm:w-auto sm:flex-row">
          <ButtonLink href={DOWNLOAD_URL}>
            <WindowsLogo weight="fill" className="size-4" aria-hidden />
            Download for Windows
          </ButtonLink>
          <ButtonLink href={REPO_URL} variant="outline">
            <GithubLogo weight="fill" className="size-4" aria-hidden />
            View on GitHub
          </ButtonLink>
        </div>
      </div>
    </Section>
  );
}
