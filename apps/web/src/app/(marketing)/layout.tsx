import { EarlyAccessBanner } from "@/components/early-access-banner";
import Footer from "@/components/sections/footer";
import Header from "@/components/sections/header";

export default function MarketingLayout({ children }: { children: React.ReactNode }) {
  return (
    <>
      <EarlyAccessBanner />
      <Header />
      <main id="main-content">{children}</main>
      <Footer />
    </>
  );
}
