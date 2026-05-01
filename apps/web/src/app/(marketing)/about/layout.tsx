import Footer from "@/components/sections/footer";
import Header from "@/components/sections/header";

interface AboutLayoutProps {
  children: React.ReactNode;
}

export default async function Layout({ children }: AboutLayoutProps) {
  return (
    <>
      <Header />
      <main>{children}</main>
      <Footer />
    </>
  );
}
