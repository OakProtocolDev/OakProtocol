import type { Metadata } from "next";
import { Providers } from "./providers";
import { PageTransition } from "@/components/PageTransition";
import "./globals.css";

export const metadata: Metadata = {
  title: "Oak Protocol | Decentralized Exchange on Arbitrum",
  description: "MEV-protected DEX with commit-reveal swaps on Arbitrum Stylus",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className="dark">
      <body className="font-sans text-oak-text-primary min-h-screen">
        <Providers>
          <PageTransition>{children}</PageTransition>
        </Providers>
      </body>
    </html>
  );
}
