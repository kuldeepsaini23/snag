import { Faq } from "@/components/faq";
import { Catalogue, Features, Video } from "@/components/features";
import { Hero } from "@/components/hero";
import { HowItWorks } from "@/components/how-it-works";
import { PrivacySection } from "@/components/privacy-section";
import { ReadyCard } from "@/components/ready-card";

// Eight parts, in the order of `parts` in src/lib/parts.ts; the top bar has a segment for each.
export default function Home() {
  return (
    <>
      <Hero />
      <Features />
      <Video />
      <Catalogue />
      <PrivacySection />
      <HowItWorks />
      <Faq />
      <ReadyCard />
    </>
  );
}
