import Link from "next/link";

import { cn } from "@/lib/utils";

export function Logo({ className }: { className?: string }) {
  return (
    <Link href="/" className={cn("flex items-center gap-2.5 rounded-lg", className)} aria-label="Snag home">
      {/* eslint-disable-next-line @next/next/no-img-element -- static export, already a 96px PNG */}
      <img src="/images/logo-96.png" alt="" width={32} height={32} className="size-8 rounded-[9px]" />
      <span className="text-[1.35rem] font-extrabold tracking-tight [font-stretch:125%]">Snag</span>
    </Link>
  );
}
