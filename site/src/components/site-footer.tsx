import Link from "next/link";

import { Logo } from "@/components/logo";
import { RELEASES_URL, REPO_URL } from "@/lib/site";

export function SiteFooter() {
  return (
    <footer className="border-t border-line">
      <div className="mx-auto flex max-w-6xl flex-col gap-8 px-4 py-10 sm:flex-row sm:items-center sm:px-6">
        <div className="flex flex-col gap-2">
          <Logo />
          <p className="text-sm text-faint">MIT © 2026 Kuldeep Saini</p>
        </div>
        <nav aria-label="Footer" className="sm:ml-auto">
          <ul className="flex flex-wrap gap-x-6 gap-y-3 text-sm text-muted">
            <li>
              <a href={REPO_URL} className="hover:text-text">
                GitHub
              </a>
            </li>
            <li>
              <Link href="/privacy/" className="hover:text-text">
                Privacy
              </Link>
            </li>
            <li>
              <a href={RELEASES_URL} className="hover:text-text">
                Changelog
              </a>
            </li>
          </ul>
        </nav>
      </div>
    </footer>
  );
}
