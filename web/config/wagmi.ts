import { getDefaultConfig } from "@rainbow-me/rainbowkit";
import { arbitrumSepolia } from "wagmi/chains";
import { http } from "wagmi";

/**
 * Wallet config for Oak Protocol.
 * - Uses only NEXT_PUBLIC_* env vars (safe for client; no private keys).
 * - Single chain (Arbitrum Sepolia) to reduce attack surface.
 */
const projectId =
  typeof process.env.NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID === "string" &&
  process.env.NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID.length > 0
    ? process.env.NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID
    : "YOUR_PROJECT_ID";

export const config = getDefaultConfig({
  appName: "Oak Protocol",
  projectId,
  chains: [arbitrumSepolia],
  ssr: true,
  transports: {
    [arbitrumSepolia.id]: http(),
  },
});
