// A link off the site (GitHub and the like): it opens in a new tab, and says so to screen readers.
// The installer link is not one of these; it stays a plain download.
export function ExternalLink({
  children,
  ...props
}: Omit<React.ComponentProps<"a">, "target" | "rel">) {
  return (
    <a {...props} target="_blank" rel="noopener noreferrer">
      {children}
      <span className="sr-only"> (opens in a new tab)</span>
    </a>
  );
}
