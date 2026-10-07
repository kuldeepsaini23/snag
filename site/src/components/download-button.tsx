import { WindowsLogo } from "@phosphor-icons/react/dist/ssr";

import { DOWNLOAD_URL } from "@/lib/site";
import { cn } from "@/lib/utils";

export function DownloadButton({ size = "lg", className }: { size?: "sm" | "lg"; className?: string }) {
  return (
    <a
      href={DOWNLOAD_URL}
      className={cn(
        "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-full bg-accent font-semibold text-on-accent transition-[filter,transform] hover:brightness-110 active:scale-[0.97]",
        size === "lg" ? "h-13 px-6 text-[1.05rem]" : "h-9 px-4 text-sm",
        className,
      )}
    >
      <WindowsLogo weight="fill" className={size === "lg" ? "size-5" : "size-4"} aria-hidden />
      {size === "lg" ? "Download for Windows" : "Download"}
    </a>
  );
}
