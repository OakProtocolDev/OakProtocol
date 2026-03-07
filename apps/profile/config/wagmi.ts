import { getDefaultConfig } from "@rainbow-me/rainbowkit";
import { arbitrum } from "wagmi/chains";
import { http } from "wagmi";

const projectId =
  typeof process.env.NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID === "string" &&
  process.env.NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID.length > 0
    ? process.env.NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID
    : "YOUR_PROJECT_ID";

export const config = getDefaultConfig({
  appName: "Oak Profile",
  projectId,
  chains: [arbitrum],
  ssr: true,
  transports: {
    [arbitrum.id]: http(),
  },
});
