import { Faq } from "@/components/faq";
import { Features } from "@/components/features";
import { GetStarted } from "@/components/get-started";
import { Hero } from "@/components/hero";
import { HowItWorks } from "@/components/how-it-works";
import { PrivacySection } from "@/components/privacy-section";
import { ScrollRail } from "@/components/scroll-rail";
import { SECTIONS } from "@/lib/site";

export default function Home() {
  return (
    <>
      <Hero />
      <Features />
      <PrivacySection />
      <HowItWorks />
      <Faq />
      <GetStarted />
      <ScrollRail sections={SECTIONS} />
    </>
  );
}
