export const SITE_URL = "https://snag.kuldeepsaini.dev";
export const REPO_URL = "https://github.com/kuldeepsaini23/snag";
export const RELEASES_URL = `${REPO_URL}/releases`;
export const ISSUES_URL = `${REPO_URL}/issues`;
export const DOWNLOAD_URL = `${REPO_URL}/releases/latest/download/Snag-Setup-1.0.0.exe`;

export const PITCH =
  "The fast, free download manager for Windows — catches any video like IDM, downloads files 8× in parallel";

// The landing page's sections, in order: the rail lists them, the header links to most of them.
export const SECTIONS = [
  { id: "top", label: "Snag" },
  { id: "features", label: "Features" },
  { id: "privacy", label: "Privacy" },
  { id: "how-it-works", label: "How it works" },
  { id: "faq", label: "FAQ" },
  { id: "get-started", label: "Get started" },
] as const;
