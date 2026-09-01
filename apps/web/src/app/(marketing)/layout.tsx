import { cookies } from "next/headers";
import { EarlyAccessBanner } from "@/components/early-access-banner";
import Footer from "@/components/sections/footer";
import Header from "@/components/sections/header";

export default async function MarketingLayout({ children }: { children: React.ReactNode }) {
  const cookieStore = await cookies();
  const bannerDismissed = cookieStore.get("allsource-product-hunt-launch-dismissed")?.value === "1";

  return (
    <>
      <EarlyAccessBanner initialDismissed={bannerDismissed} />
      <Header />
      <main id="main-content">{children}</main>
      <Footer />
    </>
  );
}
