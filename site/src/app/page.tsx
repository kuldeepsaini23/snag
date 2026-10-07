import { ClosingCta } from "@/components/closing-cta";
import { ExtensionSection } from "@/components/extension-section";
import { Faq } from "@/components/faq";
import { Features } from "@/components/features";
import { Hero } from "@/components/hero";
import { HowItWorks } from "@/components/how-it-works";
import { PrivacySection } from "@/components/privacy-section";

export default function Home() {
  return (
    <>
      <Hero />
      <Features />
      <HowItWorks />
      <PrivacySection />
      <ExtensionSection />
      <Faq />
      <ClosingCta />
    </>
  );
}
