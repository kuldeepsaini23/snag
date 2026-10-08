import Link from "next/link";

import { ExternalLink } from "@/components/external-link";
import { Logo } from "@/components/logo";
import { ISSUES_URL, RELEASES_URL, REPO_URL } from "@/lib/site";

export function SiteFooter() {
  return (
    <footer className="border-t border-line">
      <div className="mx-auto grid max-w-6xl gap-8 px-4 py-10 sm:px-6 md:grid-cols-[1fr_auto] md:items-end">
        <div className="flex flex-col gap-2">
          <Logo />
          <p className="text-sm text-faint">MIT © 2026 Kuldeep Saini</p>
          <p className="font-display text-sm text-faint italic">
            Set in Fraunces, Inter and JetBrains Mono. Snag itself is written in Rust.
          </p>
        </div>
        <nav aria-label="Footer">
          <ul className="flex flex-wrap gap-x-6 gap-y-3 text-sm text-muted">
            <li>
              <ExternalLink href={REPO_URL} className="hover:text-text">
                GitHub
              </ExternalLink>
            </li>
            <li>
              {/* Not prefetched: the static export doesn't write the file the router would prefetch, so it would 404. */}
              <Link href="/privacy/" prefetch={false} className="hover:text-text">
                Privacy
              </Link>
            </li>
            <li>
              <ExternalLink href={RELEASES_URL} className="hover:text-text">
                Changelog
              </ExternalLink>
            </li>
            <li>
              <ExternalLink href={ISSUES_URL} className="hover:text-text">
                Report a bug
              </ExternalLink>
            </li>
          </ul>
        </nav>
      </div>
    </footer>
  );
}
