"use client";

import { motion } from "framer-motion";
import Script from "next/script";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  OakSiteHeader,
} from "@oak-protocol/ui";

/** Arbitrum One chain ID for LayerZero / Stargate destination. */
const ARBITRUM_CHAIN_ID = 42161;

const container = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: { staggerChildren: 0.08 },
  },
};

const item = {
  hidden: { opacity: 0, y: 12 },
  show: { opacity: 1, y: 0 },
};

const BRIDGE_LINKS = [
  { href: "https://stargate.finance/transfer", label: "Stargate Transfer (LayerZero)", accent: true },
  { href: "https://satellite.money/", label: "Satellite (Axelar)" },
  { href: "https://bridge.arbitrum.io/", label: "Arbitrum Official Bridge" },
];

export default function BridgePage() {
  return (
    <div className="min-h-screen bg-oak-bg text-oak-text-primary">
      <OakSiteHeader current="bridge" />
      <Script
        src="https://unpkg.com/@layerzerolabs/stargate-ui@latest/element.js"
        strategy="lazyOnload"
      />
      <main className="p-4 md:p-6">
        <motion.div
          variants={container}
          initial="hidden"
          animate="show"
          className="max-w-4xl mx-auto space-y-6"
        >
          <motion.header variants={item}>
            <h1 className="text-2xl md:text-3xl font-bold tracking-tight">Bridge to Arbitrum</h1>
            <p className="text-oak-text-secondary mt-1">
              Seamless asset transfer to Arbitrum via LayerZero Stargate & Axelar · bridge.oak.trade
            </p>
          </motion.header>

          <div className="grid grid-cols-1 lg:grid-cols-5 gap-6">
            <motion.div variants={item} className="lg:col-span-2">
              <Card>
                <CardHeader>
                  <CardTitle>LayerZero Stargate</CardTitle>
                  <CardDescription>
                    Bridge assets from Ethereum, BNB Chain, Polygon, Avalanche and more to Arbitrum.
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="rounded-oak-lg border border-oak-border bg-oak-bg-elevated overflow-hidden min-h-[420px] flex items-center justify-center">
                    {/* Stargate widget: set destination to Arbitrum (42161) in widget config if supported by @layerzerolabs/stargate-ui */}
                    <stargate-widget theme="dark" data-destination-chain-id={ARBITRUM_CHAIN_ID} />
                  </div>
                </CardContent>
              </Card>
            </motion.div>
            <motion.div variants={item} className="lg:col-span-3 space-y-4">
              <Card>
                <CardHeader>
                  <CardTitle>Other bridges</CardTitle>
                  <CardDescription>Use these if you need Axelar or native Arbitrum bridges.</CardDescription>
                </CardHeader>
                <CardContent className="space-y-3">
                  {BRIDGE_LINKS.map((link) => (
                    <a
                      key={link.href}
                      href={link.href}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="block p-3 rounded-oak border border-oak-border bg-oak-bg-hover hover:border-oak-accent/50 transition-colors text-oak-text-primary"
                    >
                      <span className={link.accent ? "text-oak-accent" : ""}>{link.label} →</span>
                    </a>
                  ))}
                </CardContent>
              </Card>
            </motion.div>
          </div>
        </motion.div>
      </main>
    </div>
  );
}
