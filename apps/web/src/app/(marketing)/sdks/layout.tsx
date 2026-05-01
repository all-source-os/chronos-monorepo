import Footer from "@/components/sections/footer";
import Header from "@/components/sections/header";

interface SdksLayoutProps {
  children: React.ReactNode;
}

export default async function Layout({ children }: SdksLayoutProps) {
  return (
    <>
      <Header />
      <main>{children}</main>
      <Footer />
    </>
  );
}
