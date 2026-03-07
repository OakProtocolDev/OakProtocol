/**
 * Oak Protocol contract ABI (TP/SL/Limit orders + close position).
 * Extend with other methods (swap, addLiquidity, etc.) as needed.
 */

export const OAK_CONTRACT_ADDRESS =
  (typeof process !== "undefined" &&
    process.env?.NEXT_PUBLIC_OAK_CONTRACT_ADDRESS) ||
  "0x0000000000000000000000000000000000000000";

export const oakOrderAbi = [
  {
    name: "place_order",
    type: "function",
    stateMutability: "nonpayable",
    inputs: [
      { name: "token_in", type: "address", internalType: "address" },
      { name: "token_out", type: "address", internalType: "address" },
      { name: "amount_out", type: "uint256", internalType: "uint256" },
      { name: "trigger_price", type: "uint256", internalType: "uint256" },
      { name: "order_type", type: "uint256", internalType: "uint256" },
      { name: "oco_with_order_id", type: "uint256", internalType: "uint256" },
    ],
    outputs: [{ name: "", type: "uint256", internalType: "uint256" }],
  },
  {
    name: "cancel_order",
    type: "function",
    stateMutability: "nonpayable",
    inputs: [{ name: "order_id", type: "uint256", internalType: "uint256" }],
    outputs: [],
  },
  {
    name: "execute_order",
    type: "function",
    stateMutability: "nonpayable",
    inputs: [
      { name: "order_id", type: "uint256", internalType: "uint256" },
      { name: "min_amount_out", type: "uint256", internalType: "uint256" },
    ],
    outputs: [{ name: "", type: "uint256", internalType: "uint256" }],
  },
  {
    name: "get_order",
    type: "function",
    stateMutability: "view",
    inputs: [{ name: "order_id", type: "uint256", internalType: "uint256" }],
    outputs: [
      { name: "owner", type: "address", internalType: "address" },
      { name: "token_in", type: "address", internalType: "address" },
      { name: "token_out", type: "address", internalType: "address" },
      { name: "amount_out", type: "uint256", internalType: "uint256" },
      { name: "trigger_price", type: "uint256", internalType: "uint256" },
      { name: "order_type", type: "uint256", internalType: "uint256" },
      { name: "status", type: "uint256", internalType: "uint256" },
      { name: "created_at", type: "uint256", internalType: "uint256" },
      { name: "oco_pair", type: "uint256", internalType: "uint256" },
    ],
  },
  {
    name: "get_current_price",
    type: "function",
    stateMutability: "view",
    inputs: [
      { name: "token_in", type: "address", internalType: "address" },
      { name: "token_out", type: "address", internalType: "address" },
    ],
    outputs: [{ name: "", type: "uint256", internalType: "uint256" }],
  },
  {
    name: "get_amounts_out",
    type: "function",
    stateMutability: "view",
    inputs: [
      { name: "amount_in", type: "uint256", internalType: "uint256" },
      { name: "path", type: "address[]", internalType: "address[]" },
    ],
    outputs: [{ name: "", type: "uint256[]", internalType: "uint256[]" }],
  },
  {
    name: "swap_exact_tokens_for_tokens",
    type: "function",
    stateMutability: "nonpayable",
    inputs: [
      { name: "amount_in", type: "uint256", internalType: "uint256" },
      { name: "amount_out_min", type: "uint256", internalType: "uint256" },
      { name: "path", type: "address[]", internalType: "address[]" },
      { name: "to", type: "address", internalType: "address" },
      { name: "deadline", type: "uint256", internalType: "uint256" },
    ],
    outputs: [{ name: "", type: "uint256[]", internalType: "uint256[]" }],
  },
  {
    name: "close_position_market",
    type: "function",
    stateMutability: "nonpayable",
    inputs: [
      { name: "amount_in", type: "uint256", internalType: "uint256" },
      { name: "token_from", type: "address", internalType: "address" },
      { name: "token_to", type: "address", internalType: "address" },
      { name: "min_amount_out", type: "uint256", internalType: "uint256" },
    ],
    outputs: [{ name: "", type: "uint256", internalType: "uint256" }],
  },
  // --- Tracked positions (pro terminal) ---
  {
    name: "open_position",
    type: "function",
    stateMutability: "nonpayable",
    inputs: [
      { name: "base_token", type: "address", internalType: "address" },
      { name: "quote_token", type: "address", internalType: "address" },
      { name: "size", type: "uint256", internalType: "uint256" },
      { name: "entry_price", type: "uint256", internalType: "uint256" },
      { name: "initial_collateral", type: "uint256", internalType: "uint256" },
    ],
    outputs: [{ name: "", type: "uint256", internalType: "uint256" }],
  },
  {
    name: "add_margin",
    type: "function",
    stateMutability: "nonpayable",
    inputs: [
      { name: "position_id", type: "uint256", internalType: "uint256" },
      { name: "amount", type: "uint256", internalType: "uint256" },
    ],
    outputs: [],
  },
  {
    name: "set_position_tp_sl",
    type: "function",
    stateMutability: "nonpayable",
    inputs: [
      { name: "position_id", type: "uint256", internalType: "uint256" },
      { name: "tp_price", type: "uint256", internalType: "uint256" },
      { name: "sl_price", type: "uint256", internalType: "uint256" },
    ],
    outputs: [],
  },
  {
    name: "close_position",
    type: "function",
    stateMutability: "nonpayable",
    inputs: [
      { name: "position_id", type: "uint256", internalType: "uint256" },
      { name: "min_amount_out", type: "uint256", internalType: "uint256" },
    ],
    outputs: [{ name: "", type: "uint256", internalType: "uint256" }],
  },
  {
    name: "execute_position_tp_sl",
    type: "function",
    stateMutability: "nonpayable",
    inputs: [
      { name: "position_id", type: "uint256", internalType: "uint256" },
      { name: "min_amount_out", type: "uint256", internalType: "uint256" },
    ],
    outputs: [{ name: "", type: "uint256", internalType: "uint256" }],
  },
  {
    name: "get_position",
    type: "function",
    stateMutability: "view",
    inputs: [{ name: "position_id", type: "uint256", internalType: "uint256" }],
    outputs: [
      { name: "owner", type: "address", internalType: "address" },
      { name: "base_token", type: "address", internalType: "address" },
      { name: "quote_token", type: "address", internalType: "address" },
      { name: "size", type: "uint256", internalType: "uint256" },
      { name: "entry_price", type: "uint256", internalType: "uint256" },
      { name: "tp_price", type: "uint256", internalType: "uint256" },
      { name: "sl_price", type: "uint256", internalType: "uint256" },
      { name: "trailing_delta_bps", type: "uint256", internalType: "uint256" },
      { name: "trailing_peak_price", type: "uint256", internalType: "uint256" },
      { name: "initial_collateral", type: "uint256", internalType: "uint256" },
      { name: "margin_added", type: "uint256", internalType: "uint256" },
      { name: "opened_at", type: "uint256", internalType: "uint256" },
      { name: "status", type: "uint256", internalType: "uint256" },
    ],
  },
  {
    name: "get_position_health",
    type: "function",
    stateMutability: "view",
    inputs: [{ name: "position_id", type: "uint256", internalType: "uint256" }],
    outputs: [
      { name: "liquidation_price", type: "uint256", internalType: "uint256" },
      { name: "health_factor_bps", type: "uint256", internalType: "uint256" },
    ],
  },
  {
    name: "set_position_trailing_stop",
    type: "function",
    stateMutability: "nonpayable",
    inputs: [
      { name: "position_id", type: "uint256", internalType: "uint256" },
      { name: "trailing_delta_bps", type: "uint256", internalType: "uint256" },
    ],
    outputs: [],
  },
  {
    name: "update_trailing_stop",
    type: "function",
    stateMutability: "nonpayable",
    inputs: [
      { name: "position_id", type: "uint256", internalType: "uint256" },
      { name: "new_price", type: "uint256", internalType: "uint256" },
      { name: "min_amount_out", type: "uint256", internalType: "uint256" },
    ],
    outputs: [{ name: "", type: "uint256", internalType: "uint256" }],
  },
  {
    name: "get_next_position_id",
    type: "function",
    stateMutability: "view",
    inputs: [],
    outputs: [{ name: "", type: "uint256", internalType: "uint256" }],
  },
  {
    name: "batch_execute_positions",
    type: "function",
    stateMutability: "nonpayable",
    inputs: [
      { name: "position_ids", type: "uint256[]", internalType: "uint256[]" },
      { name: "min_amount_out", type: "uint256", internalType: "uint256" },
    ],
    outputs: [{ name: "", type: "uint256", internalType: "uint256" }],
  },
] as const;

export type OrderType = 0 | 1 | 2; // Limit, TP, SL
export type OrderStatus = 0 | 1 | 2; // Open, Executed, Cancelled

export const ORDER_TYPE_LABEL: Record<OrderType, string> = {
  0: "Limit",
  1: "TP",
  2: "SL",
};

export const ORDER_STATUS_LABEL: Record<OrderStatus, string> = {
  0: "Open",
  1: "Filled",
  2: "Cancelled",
};
