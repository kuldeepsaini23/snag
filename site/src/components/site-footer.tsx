import Link from "next/link";

import { Logo } from "@/components/logo";
import { DOWNLOAD_URL, ISSUES_URL, RELEASES_URL, REPO_URL } from "@/lib/site";

const columns = [
  {
    title: "Product",
    links: [
      { href: DOWNLOAD_URL, label: "Download for Windows" },
      { href: "/#features", label: "Features" },
      { href: RELEASES_URL, label: "Changelog" },
    ],
  },
  {
    title: "Extension",
    links: [
      { href: "/#extension", label: "Chrome, Edge, Firefox" },
      { href: "/#how-it-works", label: "Connect it to Snag" },
      { href: "/privacy/", label: "Privacy policy" },
    ],
  },
  {
    title: "Resources",
    links: [
      { href: REPO_URL, label: "Source code" },
      { href: `${REPO_URL}#build-it-yourself`, label: "Build it yourself" },
      { href: "/#faq", label: "FAQ" },
    ],
  },
  {
    title: "Community",
    links: [
      { href: ISSUES_URL, label: "Report a bug" },
      { href: `${ISSUES_URL}/new`, label: "Suggest a feature" },
      { href: REPO_URL, label: "Star on GitHub" },
    ],
  },
];

export function SiteFooter() {
  return (
    <footer className="border-t border-line">
      <div className="grid grid-cols-2 gap-px bg-line md:grid-cols-5">
        <div className="col-span-2 bg-bg px-5 py-8 sm:px-8 md:col-span-1">
          <Logo />
          <p className="mt-4 max-w-[28ch] text-sm leading-relaxed text-muted">
            The fast, free download manager for Windows.
          </p>
        </div>
        {columns.map((col) => (
          <nav key={col.title} aria-label={col.title} className="bg-bg px-5 py-8 sm:px-8">
            <p className="label text-faint">{col.title}</p>
            <ul className="mt-5 grid gap-3 text-sm">
              {col.links.map((link) => (
                <li key={link.label}>
                  {link.href.startsWith("/") ? (
                    <Link href={link.href} className="text-muted transition-colors hover:text-text">
                      {link.label}
                    </Link>
                  ) : (
                    <a href={link.href} className="text-muted transition-colors hover:text-text">
                      {link.label}
                    </a>
                  )}
                </li>
              ))}
            </ul>
          </nav>
        ))}
      </div>
      <div className="flex flex-col gap-2 border-t border-line px-5 py-4 sm:flex-row sm:justify-between sm:px-8">
        <p className="label text-faint">© 2026 Snag · MIT · Kuldeep Saini</p>
        <p className="label text-faint">Made in Rust, for everyone</p>
      </div>
    </footer>
  );
}
