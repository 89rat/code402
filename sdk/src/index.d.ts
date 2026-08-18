/**
 * @typedef {Object} CallResult
 * @property {boolean} replayed
 * @property {*} [output]
 * @property {Object} [receipt] - {receipt, commitment, signature}
 * @property {boolean} [receiptVerified]
 * @property {string} [requestId]
 * @property {string} [receiptRef]
 */

/**
 * @param {Object} opts
 * @param {string} opts.baseUrl - e.g. https://code402-edge.akrivis.workers.dev
 * @param {import('viem').LocalAccount} opts.account - payer wallet
 * @param {string} [opts.receiptSigner] - expected receipt-signing address
 * @param {number} [opts.validitySeconds]
 * @param {typeof fetch} [opts.fetchImpl]
 */
export function createClient(opts) {}

/** @type {(r: Object) => `0x${string}`} */
export function receiptCommitment(r) {}

/** @type {(args: {commitment: string, signature: string}) => Promise<`0x${string}`>} */
export function recoverReceiptSigner(args) {}

export class Code402Error extends Error {
  /** @type {number|undefined} */ status;
  /** @type {string|undefined} */ code;
  /** @type {*} */ body;
}
