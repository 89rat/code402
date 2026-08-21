import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Terminal, ShieldAlert, GitBranch } from 'lucide-react'

const SAMPLE = `$ x402check https://code402.dev/v1/tools/vat-mod97-check/call \\
    --body '{"input":{"vat_number":"GB123456789"}}'

dialect: v1-bespoke   grade: F   score: 56/100

  ✖ [blocker] non-spec-dialect
      challenge is a bespoke v1 dialect, not the spec envelope —
      official x402 clients cannot pay this endpoint
  · [minor] cors-absent
      no Access-Control-Allow-Origin — browser agents cannot call it

  Verdict: NOT PAYABLE by a conformant agent today.`

export default function Check() {
  return (
    <div className="mx-auto max-w-6xl px-6 py-16">
      <Badge variant="outline" className="border-[#27272A] font-mono text-xs text-[#06B6D4]">free brick · Apache-2.0 · zero dependencies</Badge>
      <h1 className="mt-4 text-3xl font-bold tracking-tight">x402check — can an agent actually pay you?</h1>
      <p className="mt-4 max-w-3xl leading-relaxed text-[#A1A1AA]">
        One command against any URL: a letter grade, and every finding cited to the rule it
        violates. Runs on Node 18+, in CI, or inside a Worker. No signup, no keys, no account.
      </p>

      <div className="mt-10 grid gap-6 lg:grid-cols-2">
        <Card className="border-[#27272A] bg-[#18181B]">
          <CardHeader>
            <div className="flex items-center gap-2">
              <Terminal className="h-5 w-5 text-[#06B6D4]" />
              <CardTitle className="text-base text-[#FAFAFA]">Run it</CardTitle>
            </div>
          </CardHeader>
          <CardContent>
            <pre className="overflow-x-auto rounded-md bg-[#09090B] p-4 font-mono text-xs leading-6 text-[#A1A1AAAA]">
{`# any endpoint
node cli.js https://your-api.example/call

# endpoints that validate input first
node cli.js <url> --body '{"input":{...}}'

# machine-readable, for CI
node cli.js <url> --json

# CI gate: break the build when payability breaks
x402check $URL || exit 1`}
            </pre>
            <p className="mt-4 font-mono text-xs text-[#71717A]">
              exit 0 = grade A/B · exit 1 = C/D · exit 2 = F
            </p>
          </CardContent>
        </Card>

        <Card className="border-[#27272A] bg-[#18181B]">
          <CardHeader>
            <div className="flex items-center gap-2">
              <ShieldAlert className="h-5 w-5 text-[#F59E0B]" />
              <CardTitle className="text-base text-[#FAFAFA]">We check ourselves first</CardTitle>
            </div>
          </CardHeader>
          <CardContent>
            <pre className="overflow-x-auto whitespace-pre-wrap rounded-md bg-[#09090B] p-4 font-mono text-[11px] leading-5 text-[#A1A1AA]">{SAMPLE}</pre>
            <p className="mt-4 text-xs leading-relaxed text-[#71717A]">
              Our own production endpoint grades F today — bespoke v1 dialect, v2 flip in
              progress. The report that finding generated is public in our repo's reviews/.
              That is what "measured, not claimed" means when it's uncomfortable.
            </p>
          </CardContent>
        </Card>
      </div>

      <Card className="mt-6 border-[#27272A] bg-[#18181B]">
        <CardHeader>
          <div className="flex items-center gap-2">
            <GitBranch className="h-5 w-5 text-[#06B6D4]" />
            <CardTitle className="text-base text-[#FAFAFA]">What it checks</CardTitle>
          </div>
        </CardHeader>
        <CardContent className="grid gap-3 text-sm text-[#A1A1AA] md:grid-cols-2">
          <p>· <span className="text-[#FAFAFA]">Discovery</span> — 402 + machine-readable challenge (v2 header, v1 body, bespoke dialects named)</p>
          <p>· <span className="text-[#FAFAFA]">Money fields</span> — decimal-string amounts, CAIP-2 networks, address shapes, EIP-712 domain material</p>
          <p>· <span className="text-[#FAFAFA]">Agent-UX</span> — cache hygiene, CORS/expose headers, Deprecation/Sunset on legacy dialects</p>
          <p>· <span className="text-[#FAFAFA]">Validation-first</span> — flagged as a trade-off with remediation, never a false F</p>
        </CardContent>
      </Card>
    </div>
  )
}
