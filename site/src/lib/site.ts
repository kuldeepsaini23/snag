// Server-only: read at build time. Client components get what they need as props.
import { readFileSync } from "node:fs";
import path from "node:path";

export const SITE_URL = "https://snag.kuldeepsaini.dev";
export const REPO_URL = "https://github.com/kuldeepsaini23/snag";
export const RELEASES_URL = `${REPO_URL}/releases`;
export const ISSUES_URL = `${REPO_URL}/issues`;
// Every release carries the installer under this stable name too, so the link never goes stale.
export const DOWNLOAD_URL = `${REPO_URL}/releases/latest/download/Snag-Setup.exe`;

export const PITCH =
  "The fast, free download manager for Windows, macOS and Linux — catches any video like IDM, downloads files 8× in parallel";

// The app's version, from `[workspace.package] version` in the repository's Cargo.toml.
// The build runs in site/, so that is one folder up. If a host builds site/ on its own and the
// file isn't there, the site simply shows no version number.
function readVersion(): string | null {
  try {
    const toml = readFileSync(path.join(process.cwd(), "..", "Cargo.toml"), "utf8");
    const section = toml.split(/^\[workspace\.package\]\s*$/m)[1]?.split(/^\[/m)[0] ?? "";
    return section.match(/^version\s*=\s*"([^"]+)"/m)?.[1] ?? null;
  } catch {
    return null;
  }
}

export const VERSION = readVersion();
export const INSTALLER_NAME = VERSION ? `Snag-Setup-${VERSION}.exe` : "Snag-Setup.exe";
