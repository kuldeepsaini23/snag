import Link from "next/link";

import { cn } from "@/lib/utils";

export function LogoMark({ size = 24, className }: { size?: number; className?: string }) {
  return (
    // eslint-disable-next-line @next/next/no-img-element -- static export, already a 96px PNG
    <img src="/images/logo-96.png" alt="" width={size} height={size} className={cn("rounded-[22%]", className)} />
  );
}

export function Logo({ className }: { className?: string }) {
  return (
    <Link href="/" className={cn("flex items-center gap-2.5 rounded-lg", className)} aria-label="Snag home">
      <LogoMark size={28} />
      <span className="text-lg font-bold tracking-tight">Snag</span>
    </Link>
  );
}
