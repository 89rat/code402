import { useState } from "react";
import { 
  Server, 
  Wallet, 
  CheckCircle, 
  RefreshCw,
  Send
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import type { X402State, PaymentRequirements, PaymentPayload } from "@/types/x402";

const INITIAL_REQUIREMENTS: PaymentRequirements = {
  scheme: "exact",
  network: "base",
  maxAmountRequired: "1000000", // 1 USDC
  asset: "USDC",
  merchantAddress: "0x742d35Cc6634C0532925a3b8D4C9db96590f6C7E",
  description: "Weather API — Current conditions for ZIP 94102",
  timestamp: Date.now(),
};

export function ProtocolFlowSection() {
  const [state, setState] = useState<X402State>({ tag: "Idle" });
  const [logs, setLogs] = useState<string[]>([]);
  const [requirements] = useState<PaymentRequirements>(INITIAL_REQUIREMENTS);

  const log = (msg: string) => setLogs(prev => [...prev, `> ${msg}`]);

  const reset = () => {
    setState({ tag: "Idle" });
    setLogs([]);
  };

  const step1_RequestResource = () => {
    log("GET /api/weather?zip=94102");
    log("Host: api.example.com");
    setTimeout(() => {
      log("← HTTP/2 402 Payment Required");
      setState({ tag: "PaymentRequired", requirements });
    }, 600);
  };

  const step2_ConstructPayment = () => {
    if (state.tag !== "PaymentRequired") return;
    
    const payload: PaymentPayload = {
      scheme: "exact",
      network: "base",
      amount: state.requirements.maxAmountRequired,
      asset: state.requirements.asset,
      merchantAddress: state.requirements.merchantAddress,
      timestamp: Date.now(),
      signature: "0x" + Array(130).fill("a").join(""), // mock secp256k1 sig
      payerAddress: "0xPayerAddress123456789012345678901234567890",
    };
    
    log(`Constructed payment: ${payload.amount} ${payload.asset} on ${payload.network}`);
    setState({ tag: "PaymentConstructed", payload });
  };

  const step3_Verify = () => {
    if (state.tag !== "PaymentConstructed") return;
    setState({ tag: "Verifying", payload: state.payload });
    log("POST facilitator.x402.org/verify");
    
    setTimeout(() => {
      log("← Verification: VALID");
      setState({ 
        tag: "Verified", 
        verification: {
          valid: true,
          payload: state.payload,
          requirements,
        }
      });
    }, 800);
  };

  const step4_Settle = () => {
    if (state.tag !== "Verified") return;
    setState({ tag: "Settling", verification: state.verification });
    log("POST facilitator.x402.org/settle");
    
    setTimeout(() => {
      log("← Settlement: CONFIRMED");
      log("← HTTP/2 200 OK + resource");
      setState({
        tag: "Settled",
        settlement: {
          settled: true,
          txHash: "0x" + Array(64).fill("0").map((_, i) => (i % 16).toString(16)).join(""),
          blockNumber: 18942000,
          blockTimestamp: Date.now(),
          gasUsed: "21000",
          effectiveGasPrice: "1000000000",
        },
        resource: { temp: 72, condition: "Sunny", humidity: 45 },
      });
    }, 1000);
  };

  const getStepStatus = (step: number) => {
    switch (state.tag) {
      case "Idle": return step === 1 ? "active" : "pending";
      case "PaymentRequired": return step <= 2 ? (step === 2 ? "active" : "done") : "pending";
      case "PaymentConstructed": return step <= 3 ? (step === 3 ? "active" : "done") : "pending";
      case "Verifying": return step <= 3 ? (step === 3 ? "active" : "done") : "pending";
      case "Verified": return step <= 4 ? (step === 4 ? "active" : "done") : "pending";
      case "Settling": return step <= 4 ? (step === 4 ? "active" : "done") : "pending";
      case "Settled": return "done";
      case "Error": return "error";
    }
  };

  return (
    <section id="demo" className="py-24 bg-slate-950 text-white">
      <div className="max-w-6xl mx-auto px-6">
        <div className="text-center mb-16">
          <Badge variant="outline" className="border-orange-500/30 text-orange-400 mb-4">Interactive Demo</Badge>
          <h2 className="text-3xl md:text-4xl font-bold mb-4">The x402 Payment Flow</h2>
          <p className="text-slate-400 max-w-2xl mx-auto">
            Step through a real machine-to-machine payment. No accounts. No sessions. 
            Just HTTP headers and cryptographic signatures.
          </p>
        </div>

        <div className="grid lg:grid-cols-2 gap-8">
          {/* Flow Visualization */}
          <Card className="bg-slate-900/80 border-slate-800">
            <CardHeader>
              <CardTitle className="text-white flex items-center gap-2">
                <RefreshCw className="w-5 h-5 text-orange-400" />
                Protocol State Machine
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              {/* Step 1 */}
              <FlowStep 
                status={getStepStatus(1)}
                icon={Send}
                title="1. Client Requests Resource"
                desc="GET /api/weather — no prior authentication"
              />
              
              {/* Step 2 */}
              <FlowStep 
                status={getStepStatus(2)}
                icon={Server}
                title="2. Server Responds 402 + Requirements"
                desc="PAYMENT-REQUIRED header with price, asset, network"
              />
              
              {/* Step 3 */}
              <FlowStep 
                status={getStepStatus(3)}
                icon={Wallet}
                title="3. Client Pays + Retries with Signature"
                desc="PAYMENT-SIGNATURE header with signed payload"
              />
              
              {/* Step 4 */}
              <FlowStep 
                status={getStepStatus(4)}
                icon={CheckCircle}
                title="4. Facilitator Verifies & Settles"
                desc="Server returns 200 OK + PAYMENT-RESPONSE header"
              />

              <div className="flex gap-2 pt-4">
                <Button 
                  onClick={step1_RequestResource}
                  disabled={state.tag !== "Idle"}
                  className="flex-1 bg-orange-500 hover:bg-orange-600"
                >
                  1. Request
                </Button>
                <Button 
                  onClick={step2_ConstructPayment}
                  disabled={state.tag !== "PaymentRequired"}
                  variant="outline"
                  className="flex-1 border-slate-600"
                >
                  2. Pay
                </Button>
                <Button 
                  onClick={step3_Verify}
                  disabled={state.tag !== "PaymentConstructed"}
                  variant="outline"
                  className="flex-1 border-slate-600"
                >
                  3. Verify
                </Button>
                <Button 
                  onClick={step4_Settle}
                  disabled={state.tag !== "Verified"}
                  variant="outline"
                  className="flex-1 border-slate-600"
                >
                  4. Settle
                </Button>
              </div>
              
              <Button onClick={reset} variant="ghost" className="w-full text-slate-500">
                Reset Flow
              </Button>
            </CardContent>
          </Card>

          {/* Terminal / Logs */}
          <Card className="bg-slate-900/80 border-slate-800">
            <CardHeader>
              <CardTitle className="text-white font-mono text-sm">HTTP Exchange Log</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="bg-slate-950 rounded-lg p-4 font-mono text-xs space-y-1 h-80 overflow-y-auto">
                {logs.length === 0 ? (
                  <span className="text-slate-600">// Click &quot;Request&quot; to start the flow...</span>
                ) : (
                  logs.map((log, i) => (
                    <div key={i} className={
                      log.includes("402") ? "text-orange-400" :
                      log.includes("200") ? "text-green-400" :
                      log.includes("→") ? "text-blue-400" :
                      "text-slate-400"
                    }>
                      {log}
                    </div>
                  ))
                )}
                {state.tag === "Settled" && (
                  <div className="mt-4 p-3 rounded bg-green-500/10 border border-green-500/20">
                    <div className="text-green-400 font-semibold mb-2">Resource Received:</div>
                    <pre className="text-slate-300">{JSON.stringify({
                      temp: 72,
                      condition: "Sunny", 
                      humidity: 45,
                      txHash: "0x0...0"
                    }, null, 2)}</pre>
                  </div>
                )}
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </section>
  );
}

function FlowStep({ 
  status, 
  icon: Icon, 
  title, 
  desc 
}: { 
  status: string; 
  icon: React.ElementType; 
  title: string; 
  desc: string;
}) {
  const colors = {
    pending: "text-slate-600 border-slate-800 bg-slate-900",
    active: "text-orange-400 border-orange-500/50 bg-orange-500/10",
    done: "text-green-400 border-green-500/50 bg-green-500/10",
    error: "text-red-400 border-red-500/50 bg-red-500/10",
  };

  return (
    <div className={`flex items-start gap-3 p-3 rounded-lg border ${colors[status as keyof typeof colors]}`}>
      <Icon className="w-5 h-5 mt-0.5 shrink-0" />
      <div>
        <div className="font-semibold text-sm">{title}</div>
        <div className="text-xs opacity-70">{desc}</div>
      </div>
    </div>
  );
}
