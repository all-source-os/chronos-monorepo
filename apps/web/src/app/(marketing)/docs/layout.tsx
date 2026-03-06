import Footer from "@/components/sections/footer";
import Header from "@/components/sections/header";

interface DocsLayoutProps {
  children: React.ReactNode;
}

export default async function Layout({ children }: DocsLayoutProps) {
  return (
    <>
      <Header />
      <main>{children}</main>
      <Footer />
    </>
  );
}
