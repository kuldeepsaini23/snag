# What's new in the Snag extension

Newest first. The desktop app has its own list in the repository's CHANGELOG.md.

## 1.1.0

- Subtitles come with sniffed videos: when a page's player loads its captions as separate files (.vtt, .srt, .ass), and when the page lists them as subtitle tracks, the extension sends them to Snag along with the stream, from the Download button on the page and from the popup. Snag saves them next to the video under the video's name (for example `Movie.en.vtt`), so VLC, mpv and other players show them; with Subtitles switched on in Snag's settings they also go inside the video. Needs a Snag version that supports it; older ones download the video as before.

## 1.0.3

- Snag's new copper colour: the icon, the Download button on pages and the popup.

## 1.0.2

- A new icon: an engraved hook catching a download arrow, matching the Snag app.

## 1.0.1 — 2026-10-07

### Fixed
- Links waiting for Snag no longer get stuck: a link Snag can never take (not a web link, for example) is dropped instead of holding up every link behind it. Links still wait while Snag is closed or not connected.
- At most 500 links wait for Snag; past that the oldest are dropped.
- "Grab all" works with more than 1,000 links: they go to Snag in batches of 1,000.
- "Grab all" says how many links Snag left out because they were downloaded before ("5 added, 2 already downloaded").
- When Snag can't take a download the browser started, the browser downloads it again under its original file name.
- The Download button stays in its corner and stays visible on sites whose styles used to move or hide it.
- On sites that rebuild the page without reloading (YouTube and others), the Download button no longer leaves leftover listeners behind each time the page changes.

### Security
- The Download button only responds to your own clicks; a page's script can't press it.
- A page can only ask the extension about its own tab, never about another tab.
