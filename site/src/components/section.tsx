import { cn } from "@/lib/utils";

// A section of the column: a hairline header row with mono labels, then its content.
export function Section({
  id,
  label,
  note,
  className,
  children,
}: {
  id: string;
  label: string;
  note?: string;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <section id={id} className={cn("border-t border-line", className)}>
      <div className="flex min-h-10 items-center justify-between gap-4 border-b border-line px-5 py-3 sm:px-8">
        <p className="label text-muted">{label}</p>
        {note && <p className="label hidden text-right text-faint sm:block">{note}</p>}
      </div>
      {children}
    </section>
  );
}

// The headline block that opens a section.
export function SectionIntro({ title, children }: { title: React.ReactNode; children?: React.ReactNode }) {
  return (
    <div className="px-5 pt-12 pb-14 sm:px-8 md:pt-16 md:pb-20" data-reveal>
      <h2 className="display max-w-[16ch] text-[2.4rem] sm:text-5xl md:text-[4rem]">{title}</h2>
      {children && <p className="mt-6 max-w-[54ch] text-lg leading-relaxed text-muted">{children}</p>}
    </div>
  );
}
