import { useState } from "react";
import { Copy, Check, Terminal } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";

const CODE_EXAMPLES = {
  server: `// server.ts — x402 Hono middleware (Node.js / Cloudflare Workers)
import { Hono } from "hono";
import { paymentMiddleware } from "x402-hono";

const app = new Hono();

app.use(paymentMiddleware({
  "GET /weather/:zip": {
    accepts: [{
      network: "base",
      scheme: "exact",
      maxAmount: "1000000", // 1 USDC
      asset: "USDC",
    }],
    description: "Current weather conditions",
  },
}));

app.get("/weather/:zip", (c) => {
  return c.json({ temp: 72, condition: "Sunny" });
});

export default app;`,

  client: `// client.ts — Automatic payment with x402 fetch wrapper
import { fetchWithPayment } from "@x402/fetch";
import { evm } from "@x402/evm";

const wallet = evm.walletFromPrivateKey(process.env.PRIVATE_KEY);

const response = await fetchWithPayment(
  "https://api.example.com/weather/94102",
  { method: "GET" },
  wallet
);

const weather = await response.json();
console.log(weather); // { temp: 72, condition: "Sunny" }`,

  mcp: `// mcp-server.ts — Charge per MCP tool call
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { withX402 } from "agents/x402";

const server = withX402(new McpServer({ name: "PayMCP", version: "1.0.0" }));

// Free tool
server.tool("add", "Add two numbers", { a: "number", b: "number" }, 
  async ({ a, b }) => ({ content: [{ type: "text", text: String(a + b) }] })
);

// Paid tool: $0.01 per call
server.paidTool("square", "Square a number", 0.01, { a: "number" }, {},
  async ({ a }) => ({ content: [{ type: "text", text: String(a ** 2) }] })
);`,

  rust: `// Rust client — zero-dep HTTP 402 handler
use reqwest;
use x402::{PaymentPayload, verify_signature};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    // Step 1: Request resource
    let resp = client.get("https://api.example.com/data").send().await?;
    
    if resp.status() == 402 {
        // Step 2: Parse payment requirements
        let req_hdr = resp.headers()
            .get("PAYMENT-REQUIRED")
            .ok_or("Missing header")?;
        let requirements: PaymentRequirements = 
            serde_json::from_slice(&base64::decode(req_hdr)?)?;
        
        // Step 3: Construct and sign payment
        let payload = PaymentPayload::new(&requirements, &wallet);
        let sig = payload.sign(&wallet.private_key)?;
        
        // Step 4: Retry with payment
        let result = client.get("https://api.example.com/data")
            .header("PAYMENT-SIGNATURE", base64::encode(sig))
            .send().await?;
        
        println!("{}", result.text().await?);
    }
    
    Ok(())
}`,
};

export function CodeExamplesSection() {
  const [copied, setCopied] = useState<string | null>(null);

  const copy = async (text: string, key: string) => {
    await navigator.clipboard.writeText(text);
    setCopied(key);
    setTimeout(() => setCopied(null), 2000);
  };

  return (
    <section className="py-24 bg-slate-950 text-white">
      <div className="max-w-5xl mx-auto px-6">
        <div className="text-center mb-16">
          <Badge variant="outline" className="border-green-500/30 text-green-400 mb-4">
            Implementation
          </Badge>
          <h2 className="text-3xl md:text-4xl font-bold mb-4">
            One Line to Get Paid
          </h2>
          <p className="text-slate-400 max-w-2xl mx-auto">
            Drop x402 into any HTTP server. The protocol handles negotiation, 
            verification, and settlement automatically.
          </p>
        </div>

        <Tabs defaultValue="server" className="w-full">
          <TabsList className="grid w-full grid-cols-4 bg-slate-800">
            <TabsTrigger value="server" className="data-[state=active]:bg-slate-700">
              Hono Server
            </TabsTrigger>
            <TabsTrigger value="client" className="data-[state=active]:bg-slate-700">
              Fetch Client
            </TabsTrigger>
            <TabsTrigger value="mcp" className="data-[state=active]:bg-slate-700">
              MCP Server
            </TabsTrigger>
            <TabsTrigger value="rust" className="data-[state=active]:bg-slate-700">
              Rust
            </TabsTrigger>
          </TabsList>

          {(Object.keys(CODE_EXAMPLES) as Array<keyof typeof CODE_EXAMPLES>).map((key) => (
            <TabsContent key={key} value={key} className="mt-4">
              <Card className="bg-slate-900/80 border-slate-800">
                <CardHeader className="flex flex-row items-center justify-between pb-2">
                  <CardTitle className="text-white text-sm font-mono flex items-center gap-2">
                    <Terminal className="w-4 h-4 text-slate-400" />
                    {key === "server" && "server.ts"}
                    {key === "client" && "client.ts"}
                    {key === "mcp" && "mcp-server.ts"}
                    {key === "rust" && "main.rs"}
                  </CardTitle>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => copy(CODE_EXAMPLES[key], key)}
                    className="text-slate-400 hover:text-white"
                  >
                    {copied === key ? (
                      <Check className="w-4 h-4 text-green-400" />
                    ) : (
                      <Copy className="w-4 h-4" />
                    )}
                  </Button>
                </CardHeader>
                <CardContent>
                  <pre className="font-mono text-sm text-slate-300 overflow-x-auto">
                    <code>{CODE_EXAMPLES[key]}</code>
                  </pre>
                </CardContent>
              </Card>
            </TabsContent>
          ))}
        </Tabs>
      </div>
    </section>
  );
}
