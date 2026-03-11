import type { ReactNode } from "react";
import Layout from "@theme/Layout";
import HeroSection from "@site/src/components/HeroSection/HeroSection";
import ProductDemo from "@site/src/components/ProductDemo/ProductDemo";
import FeaturesGrid from "@site/src/components/FeaturesGrid/FeaturesGrid";
import ValueProps from "@site/src/components/ValueProps/ValueProps";
import PersonaPrompts from "@site/src/components/PersonaPrompts/PersonaPrompts";

export default function Home(): ReactNode {
  return (
    <Layout description="your open source AI agent, automating engineering tasks seamlessly">
      <main>
        <HeroSection />
        <ProductDemo />
        <FeaturesGrid />
        <ValueProps />
        <PersonaPrompts />
      </main>
    </Layout>
  );
}
