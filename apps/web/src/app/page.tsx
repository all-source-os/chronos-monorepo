import { Ripple } from "@allsource/ui";
import Blog from "@/components/sections/blog";
import CTA from "@/components/sections/cta";
import FAQ from "@/components/sections/faq";
import Features from "@/components/sections/features";
import Footer from "@/components/sections/footer";
import Header from "@/components/sections/header";
import Hero from "@/components/sections/hero";
import HowItWorks from "@/components/sections/how-it-works";
import Pricing from "@/components/sections/pricing";
import Problem from "@/components/sections/problem";
import QuantIntelligence from "@/components/sections/quant-intelligence";
import SocialProof from "@/components/sections/social-proof";
import Solution from "@/components/sections/solution";
import StatStrip from "@/components/sections/stat-strip";

export default function Home() {
  return (
    <main className="relative overflow-hidden">
      {/* Ripple background effect */}
      <div className="fixed inset-0 -z-10">
        <Ripple
          mainCircleSize={300}
          mainCircleOpacity={0.1}
          numCircles={6}
          className="opacity-50"
        />
      </div>

      <Header />
      <Hero />
      {/* Stats demoted below the fold — final values painted, never "0K" flash */}
      <StatStrip />
      {/* Logos section hidden - needs real partner logos */}
      {/* <Logos /> */}
      <Problem />
      <Solution />
      <HowItWorks />
      {/* Testimonials carousel hidden - no real testimonials yet */}
      {/* <TestimonialsCarousel /> */}
      <Features />
      <SocialProof />
      <QuantIntelligence />
      <Pricing />
      <FAQ />
      <Blog />
      <CTA />
      <Footer />
    </main>
  );
}
