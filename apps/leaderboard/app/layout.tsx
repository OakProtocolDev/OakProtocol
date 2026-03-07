import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Leaderboard | Oak Protocol",
  description: "Top Traders and Top Farmers by Volume and Recency",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <body className="min-h-screen font-sans">{children}</body>
    </html>
  );
}
