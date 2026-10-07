# What's new in the Snag extension

Newest first. The desktop app has its own list in the repository's CHANGELOG.md.

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
