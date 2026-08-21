/**
 * x402 Protocol Type Definitions
 * Version: x402 spec v1.0.0 (2025-09-23)
 * 
 * Deterministic type system for the x402 payment protocol.
 * All monetary values represented as atomic units (wei/lamports/stroops)
 * to eliminate floating-point soundness holes.
 */

// ─────────────────────────────────────────────────────────────────────────────
// Core Protocol Types
// ─────────────────────────────────────────────────────────────────────────────

export type NetworkId = 
  | "base" 
  | "base-sepolia"
  | "ethereum" 
  | "polygon" 
  | "optimism" 
  | "arbitrum" 
  | "avalanche"
  | "solana"
  | "aptos"
  | "stellar"
  | "sui"
  | "hedera";

export type Scheme = "exact" | "upto";

export type AssetType = "USDC" | "USDT" | "ETH" | "SOL" | "STRK";

// Payment requirement returned by server on 402 response
export interface PaymentRequirements {
  readonly scheme: Scheme;
  readonly network: NetworkId;
  readonly maxAmountRequired: string; // atomic units, base-10 string
  readonly asset: AssetType;
  readonly merchantAddress: string;
  readonly description: string;
  readonly timestamp: number; // Unix epoch ms
  readonly expiresAt?: number;
  readonly metadata?: Record<string, string>;
}

// Client-constructed payment payload
export interface PaymentPayload {
  readonly scheme: Scheme;
  readonly network: NetworkId;
  readonly amount: string; // atomic units
  readonly asset: AssetType;
  readonly merchantAddress: string;
  readonly timestamp: number;
  readonly signature: string; // hex-encoded
  readonly payerAddress: string;
  readonly nonce?: string;
}

// Facilitator verification response
export interface VerificationResponse {
  readonly valid: boolean;
  readonly payload: PaymentPayload;
  readonly requirements: PaymentRequirements;
  readonly error?: string;
}

// Settlement confirmation
export interface SettlementResponse {
  readonly settled: boolean;
  readonly txHash: string;
  readonly blockNumber?: number;
  readonly blockTimestamp?: number;
  readonly gasUsed?: string;
  readonly effectiveGasPrice?: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Protocol State Machine
// ─────────────────────────────────────────────────────────────────────────────

export type X402State =
  | { tag: "Idle" }
  | { tag: "PaymentRequired"; requirements: PaymentRequirements }
  | { tag: "PaymentConstructed"; payload: PaymentPayload }
  | { tag: "Verifying"; payload: PaymentPayload }
  | { tag: "Verified"; verification: VerificationResponse }
  | { tag: "Settling"; verification: VerificationResponse }
  | { tag: "Settled"; settlement: SettlementResponse; resource: unknown }
  | { tag: "Error"; error: string };

// ─────────────────────────────────────────────────────────────────────────────
// HTTP Protocol Types
// ─────────────────────────────────────────────────────────────────────────────

export const X402_HEADERS = {
  PAYMENT_REQUIRED: "PAYMENT-REQUIRED",
  PAYMENT_SIGNATURE: "PAYMENT-SIGNATURE",
  PAYMENT_RESPONSE: "PAYMENT-RESPONSE",
} as const;

export interface X402HTTPResponse<T = unknown> {
  readonly status: 200 | 402;
  readonly headers: Record<string, string>;
  readonly body: T;
}

// ─────────────────────────────────────────────────────────────────────────────
// Configuration Types
// ─────────────────────────────────────────────────────────────────────────────

export interface FacilitatorConfig {
  readonly url: string;
  readonly timeoutMs: number;
  readonly retries: number;
}

export interface X402ClientConfig {
  readonly wallet: {
    readonly privateKey: string;
    readonly address: string;
  };
  readonly facilitator: FacilitatorConfig;
  readonly supportedNetworks: readonly NetworkId[];
}

export interface X402ServerConfig {
  readonly merchantAddress: string;
  readonly acceptedAssets: readonly AssetType[];
  readonly acceptedNetworks: readonly NetworkId[];
  readonly schemes: readonly Scheme[];
  readonly facilitatorUrl?: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Network Constants (empirical, version-anchored 2025-09-23)
// ─────────────────────────────────────────────────────────────────────────────

export const ATOMIC_UNITS: Record<NetworkId, number> = {
  "base": 6,            // USDC = 10^6
  "base-sepolia": 6,
  "ethereum": 18,       // ETH = 10^18
  "polygon": 6,
  "optimism": 6,
  "arbitrum": 6,
  "avalanche": 6,
  "solana": 9,          // SOL = 10^9
  "aptos": 8,
  "stellar": 7,
  "sui": 9,
  "hedera": 8,
};

export const FACILITATOR_DEFAULT = "https://x402.org/facilitator";
