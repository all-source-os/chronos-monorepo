import Footer from "@/components/sections/footer";
import Header from "@/components/sections/header";

export default function MarketingLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="marketing-theme min-h-screen bg-background text-foreground">
      <Header />
      <main id="main-content">{children}</main>
      <Footer />
    </div>
  );
}
