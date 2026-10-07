import { DownloadButton } from "@/components/download-button";
import { SegmentMap } from "@/components/segment-bar";

export function ClosingCta() {
  return (
    <section className="mx-auto max-w-6xl px-4 py-28 sm:px-6 md:py-36">
      <SegmentMap progress={[1, 1, 1, 1, 1, 1, 1, 1]} className="h-2.5" />
      <div className="mt-10 flex flex-col gap-8 md:flex-row md:items-end md:justify-between">
        <h2 className="display max-w-[12ch] text-5xl md:text-7xl">Grab your first download</h2>
        <div className="flex flex-col gap-3 md:items-end">
          <DownloadButton />
          <p className="text-sm text-muted">Free and open source · Windows 10 &amp; 11 · 12 MB</p>
        </div>
      </div>
    </section>
  );
}
