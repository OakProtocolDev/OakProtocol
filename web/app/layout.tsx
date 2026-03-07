import type { Metadata } from "next";
import { Inter, JetBrains_Mono } from "next/font/google";
import { Providers } from "./providers";
import { PageTransition } from "@/components/PageTransition";
import "./globals.css";

const inter = Inter({
  subsets: ["latin"],
  variable: "--font-inter",
  display: "swap",
});

const jetbrainsMono = JetBrains_Mono({
  subsets: ["latin"],
  variable: "--font-jetbrains-mono",
  display: "swap",
});

export const metadata: Metadata = {
  title: "Oak Stylus Trading Engine | Arbitrum",
  description: "Native Stylus trading terminal with limit orders, TP/SL and trailing stops on Arbitrum",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className="dark">
      <body className={`${inter.variable} ${jetbrainsMono.variable} font-sans text-oak-text-primary min-h-screen`}>
        <Providers>
          <PageTransition>{children}</PageTransition>
        </Providers>
      </body>
    </html>
  );
}
