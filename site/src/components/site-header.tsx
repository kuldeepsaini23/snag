"use client";

import { GithubLogo, WindowsLogo } from "@phosphor-icons/react";

import { Logo } from "@/components/logo";
import { DOWNLOAD_URL, REPO_URL } from "@/lib/site";
import { useActiveSection } from "@/lib/use-active-section";
import { cn } from "@/lib/utils";

const links = [
  { id: "features", label: "Features" },
  { id: "privacy", label: "Privacy" },
  { id: "how-it-works", label: "How it works" },
  { id: "faq", label: "FAQ" },
];

export function SiteHeader() {
  const active = useActiveSection(links.map((link) => link.id));

  return (
    <header className="sticky top-0 z-40 flex h-(--header-h) items-stretch border-b border-line bg-bg/85 backdrop-blur-md">
      <Logo className="border-r border-line px-4 sm:px-5" />
      <nav aria-label="Main" className="hidden items-center gap-1 px-4 md:flex">
        {links.map((link) => (
          <a
            key={link.id}
            href={`/#${link.id}`}
            aria-current={active === link.id ? "true" : undefined}
            className={cn(
              "label px-2 py-1.5 transition-colors",
              active === link.id ? "bg-text text-bg" : "text-muted hover:text-text",
            )}
          >
            {link.label}
          </a>
        ))}
      </nav>
      <div className="ml-auto flex items-stretch">
        <a
          href={REPO_URL}
          aria-label="Snag on GitHub"
          className="grid w-(--header-h) place-items-center border-l border-line text-muted transition-colors hover:bg-white/5 hover:text-text"
        >
          <GithubLogo weight="fill" className="size-[18px]" aria-hidden />
        </a>
        <a
          href={DOWNLOAD_URL}
          className="flex items-center gap-2 bg-accent px-4 font-mono text-xs font-medium tracking-[0.12em] text-on-accent uppercase transition-[filter] hover:brightness-110 sm:px-5"
        >
          <WindowsLogo weight="fill" className="size-4" aria-hidden />
          Download
        </a>
      </div>
    </header>
  );
}
