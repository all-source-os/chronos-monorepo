import Footer from "@/components/sections/footer";
import Header from "@/components/sections/header";

interface StatusLayoutProps {
  children: React.ReactNode;
}

export default async function Layout({ children }: StatusLayoutProps) {
  return (
    <>
      <Header />
      <main>{children}</main>
      <Footer />
    </>
  );
}
