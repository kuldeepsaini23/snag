const test = require("node:test");
const assert = require("node:assert");
const { isVideoPage } = require("../catch-rules.js");

const yes = [
  ["www.youtube.com", "/watch"], ["www.youtube.com", "/shorts/abc"], ["x.com", "/user/status/1"],
  ["www.instagram.com", "/reel/xyz/"], ["www.tiktok.com", "/@a/video/1"], ["vimeo.com", "/123456"],
  ["www.reddit.com", "/r/videos/comments/abc/title/"], ["www.facebook.com", "/watch"], ["www.facebook.com", "/reel/123"],
  ["www.twitch.tv", "/videos/123"], ["www.twitch.tv", "/user/clip/Abc"], ["soundcloud.com", "/artist/track"],
  ["www.dailymotion.com", "/video/x8abc"],
];
const no = [
  ["www.youtube.com", "/"], ["www.youtube.com", "/@channel"], ["x.com", "/home"], ["vimeo.com", "/about"],
  ["www.reddit.com", "/r/videos/"], ["www.facebook.com", "/profile.php"], ["www.twitch.tv", "/someuser"],
  ["soundcloud.com", "/artist"], ["www.dailymotion.com", "/"],
];

test("shows the button on single video pages", () => {
  for (const [h, p] of yes) assert.strictEqual(isVideoPage(h, p), true, `${h}${p}`);
});

test("hides it on feeds, profiles and home pages", () => {
  for (const [h, p] of no) assert.strictEqual(isVideoPage(h, p), false, `${h}${p}`);
});
