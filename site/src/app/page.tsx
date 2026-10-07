import { Faq } from "@/components/faq";
import { Features } from "@/components/features";
import { Hero } from "@/components/hero";
import { HowItWorks } from "@/components/how-it-works";
import { PrivacySection } from "@/components/privacy-section";
import { ReadyCard } from "@/components/ready-card";
import { SpeedLine } from "@/components/speed-line";

export default function Home() {
  return (
    <>
      <Hero />
      <Features />
      <SpeedLine id="after-features" duration={22} className="mt-20 h-14 opacity-45 md:mt-24" />
      <PrivacySection />
      <HowItWorks />
      <SpeedLine id="after-steps" duration={26} className="mt-20 h-14 opacity-45 md:mt-24" />
      <Faq />
      <ReadyCard />
    </>
  );
}
