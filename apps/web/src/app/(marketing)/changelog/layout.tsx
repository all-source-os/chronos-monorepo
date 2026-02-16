import Footer from "@/components/sections/footer";
import Header from "@/components/sections/header";

interface ChangelogLayoutProps {
  children: React.ReactNode;
}

export default async function Layout({ children }: ChangelogLayoutProps) {
  return (
    <>
      <Header />
      <main>{children}</main>
      <Footer />
    </>
  );
}
