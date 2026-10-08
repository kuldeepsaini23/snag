# What's new in Snag

Newest first. Snag shows the newest entry once after an update; the Help menu (?) has the full list.

## 1.1.0 — 2026-10-08

### New
- Snag now runs on Linux (AppImage and .deb) and macOS (Apple silicon and Intel), as well as Windows.
- On Linux: the tray icon, desktop notifications, start at login, and in-app updates for the AppImage.
- On macOS: a menu bar icon, Notification Center, start at login, the native window buttons, ⌘ shortcuts, and in-app updates.
- Settings → Appearance → Animations now says Follow system on Linux and macOS.

### Improved
- Smaller images in the app and on the website.

## 1.0.3 — 2026-10-08

### Improved
- A new logo: an engraved hook catching a download arrow, with a bold version for small places like the tray.
- Snag's colour is now copper (it was a yellow-orange). If you picked your own accent colour, it stays.
- The taskbar and tray icons follow your accent colour, like the logo in the window.
- A Windows notification tells you when an update is ready, even while Snag runs hidden in the tray.
- The update card shows on every screen and before other messages, and the ? menu has Check for updates.
- Settings → Appearance → Animations: On, Off, or Follow Windows.

## 1.0.2 — 2026-10-07

### Fixed
- YouTube downloads no longer fail with "The page needs to be reloaded": when the browser's cookies have gone stale, Snag tries again without them.

## 1.0.1 — 2026-10-07

### Fixed
- Dialogs no longer turn black after a while when Snag draws with the CPU (the default).
- Scrolling is about 3 times smoother with CPU drawing, and lists no longer leave trails behind.
- "Grab all" skips links you've already downloaded and says how many, instead of a popup for each.
- A "move to folder" rule says "copied" when the original file is in use and has to stay.
- Saved web pages no longer pack whole videos into the .html file.
- Links that differ only in "ref" or "share" count as different downloads outside YouTube.
- Pausing a page save no longer leaves a second copy behind.

### Improved
- Snag updates itself: when a newer version is out, a card offers it, and Update now downloads it, checks it against its published checksum, installs it and reopens Snag. Switch it off in Settings → General.
- Report a bug → Report on GitHub opens a filled-in GitHub issue in your browser.
- Send from your phone uses its own code (the extension's never travels over Wi-Fi) and listens only on your home network.
- The browser extension is 1.0.1: steadier Download button, "Grab all" of more than 1,000 links, and links waiting for Snag never get stuck.

## 1.0.0 — 2026-10-07 (first release)

### Downloads
- Fast downloads: big files are fetched over several connections at once, and pause and resume where they stopped, even after a restart.
- Videos and music from 1,000+ sites (YouTube and many more): pick the quality or MP3, or let Snag use your favourite quality straight away. Whole playlists too, with the videos you choose.
- Subtitles for videos, including auto-generated ones, built into the file.
- Record live streams: Stop keeps everything recorded so far.
- Pictures from image pages (Pinterest, Imgur, Reddit galleries, Instagram and X photo posts) are saved into a folder each.
- Torrents and magnet links download in the same list, with the same queues, pause and speed limit.
- A GitHub repository link downloads the code as a ZIP.
- ffmpeg (for HD video, MP3 and conversions) is fetched once when first needed (35 MB) and checked against its published checksum.
- Some video streams hide their pieces as images; Snag repairs them into a video that plays.
- Failed downloads try again by themselves after a timeout or a busy server.
- "Refresh link": give an expired or stopped download a fresh link and it carries on.
- After-download rules: convert to MP3, make a video smaller, unpack .zip/.7z, or move files to a folder, by file type, site or category.
- Save a web page as one offline file, with its styles and pictures inside.
- Already downloaded? Snag says so instead of downloading the same thing twice.
- Safety check: programs you download can be looked up on VirusTotal with your own free key (only a fingerprint of the file is sent).

### Browser extension
- Send any download from Chrome, Edge or Firefox to Snag. Connecting takes one click: press Connect in the extension, then Allow in Snag.
- On a video page the Snag button opens a quality menu, and what the player is already playing starts at once.
- Finds videos and streams on any page, and "Grab all" sends every link and picture on a page in one go.
- Right-click a picture to send the picture itself.
- The extension follows Snag's accent colour.
- Put the Download button in any corner (drag it), or hide it on sites you choose.
- Links sent while Snag is closed wait in the extension and go to Snag when it opens.

### Around the app
- Copy a link anywhere and Snag offers to download it.
- Send from your phone: scan a QR code on your home Wi-Fi, then paste or share links on the phone and they download on the PC.
- Watch channels and playlists: new uploads download by themselves.
- Queues with schedules and a speed limit, for downloads at night or in the background.
- Closing the window keeps Snag downloading in the tray, and a Windows notification says when downloads finish or fail.
- A new look: animated tabs, panels and sheets, a cleaner toolbar, thumbnails for videos, and Cancel for downloads you no longer want.
- Pick any accent colour from the swatches, a hex value or the colour picker; the logo and tiles follow it.
- Light theme, or follow Windows.
- Stats: what you downloaded per day, by type and by site.
- Grid view for videos and pictures, a live speed graph, and a map of each download's connections.
- Torrents and saved web pages have their own sections.
- Drop links, .torrent files or a .txt list of links onto the window.
- A short tour on first start, which also asks what Snag may use on your PC.
- A Help menu (?) with keyboard shortcuts, help, this list, and Report a bug, which saves a report on your Desktop and sends nothing anywhere.

### Fixed
- Thumbnails that didn't show for some finished YouTube videos now load, and a thumbnail that fails to load is tried again.
