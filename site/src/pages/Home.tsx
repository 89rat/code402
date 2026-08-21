import { Navbar } from "@/sections/Navbar";
import { HeroSection } from "@/sections/HeroSection";
import { ProtocolFlowSection } from "@/sections/ProtocolFlowSection";
import { SystemsAnalysisSection } from "@/sections/SystemsAnalysisSection";
import { CodeExamplesSection } from "@/sections/CodeExamplesSection";
import { FooterSection } from "@/sections/FooterSection";

export default function Home() {
  return (
    <div className="min-h-screen bg-slate-950 text-white">
      <Navbar />
      <HeroSection />
      <ProtocolFlowSection />
      <SystemsAnalysisSection />
      <div id="code">
        <CodeExamplesSection />
      </div>
      <FooterSection />
    </div>
  );
}
