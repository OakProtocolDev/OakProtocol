import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Bridge | Oak Protocol",
  description: "Bridge assets to Arbitrum via LayerZero Stargate and Axelar",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <body className="min-h-screen font-sans antialiased">{children}</body>
    </html>
  );
}
