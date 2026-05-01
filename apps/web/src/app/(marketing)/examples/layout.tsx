import Footer from "@/components/sections/footer";
import Header from "@/components/sections/header";

interface ExamplesLayoutProps {
  children: React.ReactNode;
}

export default async function Layout({ children }: ExamplesLayoutProps) {
  return (
    <>
      <Header />
      <main>{children}</main>
      <Footer />
    </>
  );
}
