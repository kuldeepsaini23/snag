# Snag privacy policy

Effective 7 October 2026

For the Snag desktop app and the Snag browser extension for Chrome, Edge and Firefox.

## In short

Snag has no account, no analytics, no ads and no server of its own. The extension sends what it reads only to the Snag app on the same computer. Nothing is sent to the developer or to any third party, and nothing is sold.

## The browser extension
The extension exists to hand downloads to the Snag app on your computer. To do that, it reads:

- the addresses (URLs), types and sizes of the media your open tabs load, so it can offer to download the video or song that is playing;
- the address of the page you are on;
- **only when you send a download**: the cookies of the site you are downloading from, and that page’s address as the referrer, because many sites refuse a download without them;
- **only when you open Grab from page**: the links, images and media on that page, so you can pick which ones to download.

All of this goes **only to the Snag app on the same computer**, at 127.0.0.1, and only after you have allowed the connection once in Snag. It is never sent to the developer or to anyone else. The extension contains no analytics, no tracking and no ads.

The extension keeps a few things in your browser’s own extension storage:

- the pairing code that lets it talk to Snag;
- your choices: where the Download button sits, the sites you have hidden it on, and Snag’s accent colour;
- links you sent while Snag was closed, until Snag opens and takes them;
- the list of media found in each open tab, which is forgotten when you close the tab.

Removing the extension removes all of it.

Why it asks for each browser permission:

- **Access to the sites you visit** and **web requests**: to notice the media a page loads and show the Download button on it.
- **Cookies**: to pass the current site’s cookies to Snag when you send a download from it.
- **Downloads**: to hand downloads you start in the browser over to Snag.
- **Scripting**: to list a page’s links and images when you open Grab from page.
- **Context menus**, **storage** and **alarms**: for the right-click “Download with Snag” items, the settings above, and sending links that waited while Snag was closed.

## The Snag app
Snag keeps your download list, history and settings in %APPDATA%\Snag on your PC. Downloaded files go to the folders you choose. None of it is uploaded anywhere.

Snag connects only to:

- the sites you download from;
- **GitHub**, to fetch and update its helpers yt-dlp (for video sites) and gallery-dl (for image galleries), and to look for a newer Snag (at start and every few hours; nothing about you is sent, and you can switch it off in Settings → General). An update is only installed when you click Update now, and only if it matches the checksum published with it;
- **gyan.dev**, once, to fetch ffmpeg the first time you need it for HD video or MP3;
- **VirusTotal**, only if you add your own API key, and then only the downloaded file’s SHA-256 fingerprint is sent, never the file itself.

**Clipboard watch**, if you leave it on, looks at text you copy on your PC to spot links. That text stays on your PC.

**Phone sharing** is off by default. When you turn it on, phones on your own local network can send links to Snag.

**Bug reports** are a text file Snag writes on your PC. Nothing is sent automatically: you read it and choose whether to share it. It leaves out your pairing code, cookies, keys and personal folder names.

## What Snag never does

- No accounts, sign-ins or email addresses.
- No analytics, telemetry, tracking or advertising.
- No selling or sharing of your data, because none of it reaches the developer.

Snag is open source, so you can check all of this in the [source code](https://github.com/kuldeepsaini23/snag).

## This website
snag.kuldeepsaini.dev is a static site with no cookies, analytics or trackers. The company that hosts it may keep standard server logs, such as IP addresses, for security.

## Changes and contact
If this policy changes, the new version will be posted here with a new effective date, and the change will be visible in the project’s history on GitHub.

Questions or concerns? Open an issue at [github.com/kuldeepsaini23/snag/issues](https://github.com/kuldeepsaini23/snag/issues).

_This is the same policy as https://snag.kuldeepsaini.dev/privacy._
