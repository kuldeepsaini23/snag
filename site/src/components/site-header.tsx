import { GithubLogo } from "@phosphor-icons/react/dist/ssr";

import { DownloadButton } from "@/components/download-button";
import { Logo } from "@/components/logo";
import { REPO_URL } from "@/lib/site";

const links = [
  { href: "/#features", label: "Features" },
  { href: "/#how-it-works", label: "How it works" },
  { href: "/#privacy", label: "Privacy" },
  { href: "/#faq", label: "FAQ" },
];

export function SiteHeader() {
  return (
    <header className="sticky top-0 z-40 border-b border-line/70 bg-bg/80 backdrop-blur-md">
      <div className="mx-auto flex h-16 max-w-6xl items-center gap-6 px-4 sm:px-6">
        <Logo />
        <nav aria-label="Main" className="hidden md:block">
          <ul className="flex items-center gap-1 text-sm text-muted">
            {links.map((link) => (
              <li key={link.href}>
                <a href={link.href} className="rounded-full px-3 py-2 transition-colors hover:text-text">
                  {link.label}
                </a>
              </li>
            ))}
          </ul>
        </nav>
        <div className="ml-auto flex items-center gap-2">
          <a
            href={REPO_URL}
            className="grid size-9 place-items-center rounded-full text-muted transition-colors hover:bg-raised hover:text-text"
            aria-label="Snag on GitHub"
          >
            <GithubLogo weight="fill" className="size-5" aria-hidden />
          </a>
          <DownloadButton size="sm" />
        </div>
      </div>
    </header>
  );
}
