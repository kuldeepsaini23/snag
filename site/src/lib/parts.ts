// The landing page's eight sections, in order. The top bar has one segment per part: each fills
// as you read through its section, and the bar's label names the part you are in.
export const parts = [
  { id: "top", name: "Snag" },
  { id: "features", name: "Downloads" },
  { id: "video", name: "Video" },
  { id: "catalogue", name: "Catalogue" },
  { id: "privacy", name: "Privacy" },
  { id: "how-it-works", name: "Setup" },
  { id: "faq", name: "Questions" },
  { id: "get-started", name: "Download" },
] as const;

export type PartId = (typeof parts)[number]["id"];

export function partNumber(id: PartId) {
  return parts.findIndex((part) => part.id === id) + 1;
}

export const pad = (n: number) => String(n).padStart(2, "0");
