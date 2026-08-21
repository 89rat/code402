import { Github, ExternalLink, Mail } from "lucide-react";

export function FooterSection() {
  return (
    <footer className="py-12 bg-slate-950 border-t border-slate-800 text-slate-400">
      <div className="max-w-6xl mx-auto px-6">
        <div className="grid md:grid-cols-3 gap-8 mb-8">
          <div>
            <div className="text-2xl font-bold text-white mb-2">x402</div>
            <p className="text-sm text-slate-500">
              Internet-native payments for the agentic web. 
              Open standard. No accounts required.
            </p>
          </div>
          <div>
            <div className="font-semibold text-white mb-3">Resources</div>
            <ul className="space-y-2 text-sm">
              <li>
                <a href="https://x402.org" target="_blank" rel="noopener noreferrer" className="hover:text-orange-400 flex items-center gap-1">
                  Protocol Spec <ExternalLink className="w-3 h-3" />
                </a>
              </li>
              <li>
                <a href="https://github.com/x402-foundation/x402" target="_blank" rel="noopener noreferrer" className="hover:text-orange-400 flex items-center gap-1">
                  GitHub <Github className="w-3 h-3" />
                </a>
              </li>
              <li>
                <a href="https://developers.cloudflare.com/agents/tools/payments/x402/" target="_blank" rel="noopener noreferrer" className="hover:text-orange-400 flex items-center gap-1">
                  Cloudflare Docs <ExternalLink className="w-3 h-3" />
                </a>
              </li>
            </ul>
          </div>
          <div>
            <div className="font-semibold text-white mb-3">Network</div>
            <ul className="space-y-2 text-sm">
              <li>Base (Mainnet + Sepolia)</li>
              <li>Ethereum</li>
              <li>Polygon, Optimism, Arbitrum</li>
              <li>Solana, Aptos, Sui, Stellar</li>
            </ul>
          </div>
        </div>
        <div className="pt-8 border-t border-slate-800 flex flex-col md:flex-row justify-between items-center gap-4 text-sm text-slate-600">
          <div>
            code402.dev — Systems Architecture Reference
          </div>
          <div className="flex items-center gap-4">
            <span>x402 spec v1.0.0 (2025-09-23)</span>
            <a href="mailto:x402@cloudflare.com" className="hover:text-slate-400">
              <Mail className="w-4 h-4" />
            </a>
          </div>
        </div>
      </div>
    </footer>
  );
}
