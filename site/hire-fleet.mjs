#!/usr/bin/env node
// ============================================================
// Paperclip fleet hiring — Dr J
// ONE company: Code402 (code402.dev). Nothing else.
// CEO: kimi (Master of All Trades) · Board: zai (The Contrarian)
// Founder & CTO: claude · Workers: the OpenFang fleet
// Every title = 80% primary duty + 20% overlap duty.
// Heartbeats stay OFF at hire — no token burn until runtimes wired.
//
// Usage:   node hire-fleet.mjs          (server must be running)
//          node hire-fleet.mjs --dry    (print plan, change nothing)
// Server:  npx paperclipai run    → http://127.0.0.1:3100
// ============================================================
const BASE = process.env.PAPERCLIP_API || "http://127.0.0.1:3100/api";
const DRY = process.argv.includes("--dry");

const ANTHEM = "Flag & anthem: Profit. Perfection. Reputation. Longevity. Excellence. Virality — the heartbeat and soul of all.";
const MISSION = "The proof layer for machine payments. x402 marketplace + XDR-1 receipts + trust certification. Make Code402 the most profitable company in x402, built to last.";

async function api(method, path, body) {
  const r = await fetch(BASE + path, {
    method,
    headers: { "content-type": "application/json" },
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await r.text();
  let data; try { data = JSON.parse(text); } catch { data = text; }
  if (!r.ok) throw new Error(`${method} ${path} -> ${r.status}: ${text.slice(0, 300)}`);
  return data;
}

async function companyUpdate(id, patch) {
  try { return await api("PATCH", `/companies/${id}`, patch); }
  catch { return await api("PUT", `/companies/${id}`, patch); }
}

// Paperclip role enum: ceo|cto|cmo|cfo|security|engineer|designer|pm|qa|devops|researcher|general
// name, role, title (80% primary + 20% overlap), manager(name|null), budget cents
const ROSTER = [
  // ---- Top ----
  ["kimi","ceo","CEO - Master of All Trades. 100% generalist: strategy, arbitration, final routing across all departments",null,5000],
  ["zai","general","Board Advisor - The Contrarian. Challenges every plan with contrary ideas, measured on outperforming consensus. Red-teams strategy before the board sees it",null,2500],
  ["claude","cto","Founder and CTO. 80% proof-layer platform: marketplace, XDR-1 receipts, certification. 20% overlap: corporate strategy with CEO","kimi",10000],
  ["openclaw-main","general","Chief of Staff. 80% approvals triage + cross-dept unblock, 20% overlap: CEO routing","kimi",2500],
  ["orchestrator","pm","COO - Task Routing and Governance. 80% orchestration across pods, 20% overlap: growth pipeline","kimi",2500],
  // ---- Engineering (under claude) ----
  ["architect","engineer","Architecture. 80% system design, 20% overlap: security","claude",1000],
  ["coder","engineer","Implementation. 80% features, 20% overlap: tests","claude",1000],
  ["code-reviewer","engineer","Review. 80% code review, 20% overlap: security audit","claude",1000],
  ["debugger","engineer","Debugging. 80% defects, 20% overlap: tests","claude",1000],
  ["test-engineer","qa","Testing. 80% test suites, 20% overlap: review","claude",1000],
  ["devops-lead","devops","DevOps. 80% infra/deploys/uptime, 20% overlap: security","claude",1000],
  ["security-auditor","security","Security. 80% audits, 20% overlap: trust analysis","claude",1000],
  ["doc-writer","engineer","Docs. 80% technical docs + llms.txt/manifests, 20% overlap: devrel content","claude",1000],
  // ---- Trust (under claude) ----
  ["judge","qa","Trust Analyst lead. 80% Drift Wall + weekly x402 reliability report, 20% overlap: security audit","claude",2500],
  ["claim-auditor","qa","Certification checks. 80% x402 conformance verification, 20% overlap: legal","judge",1000],
  ["legal-assistant","general","Legal. 80% seller agreements + compliance (non-custodial always), 20% overlap: certification","judge",1000],
  // ---- Growth (under kimi) ----
  ["prospector-elite","general","Growth lead. 80% API-seller acquisition pipeline, 20% overlap: devrel","kimi",2500],
  ["outreach","general","Seller outreach. 80% first-touch sequences, 20% overlap: support","prospector-elite",1000],
  ["sales-assistant","general","Sales ops. 80% pipeline hygiene, 20% overlap: outreach","prospector-elite",1000],
  ["key-account-manager","general","Key accounts. 80% top sellers + certification upsells, 20% overlap: support","prospector-elite",1000],
  ["customer-support","general","Support. 80% agent/seller issues, 20% overlap: sales handoff","prospector-elite",1000],
  ["pricing-elasticity","cfo","Pricing + finance. 80% call pricing, take rates, cert tiers, 20% overlap: analysis","prospector-elite",1000],
  ["market-universe","researcher","Market mapping. 80% x402 seller universe/TAM, 20% overlap: research","prospector-elite",1000],
  ["recruiter","general","Recruiting. 80% partner/talent pipeline, 20% overlap: outreach","prospector-elite",1000],
  // ---- DevRel & Distribution (under writer) ----
  ["writer","cmo","DevRel lead. 80% Bazaar/MCP-registry/x402.org distribution + content, 20% overlap: social","kimi",2500],
  ["social-media","general","Social. 80% brand channels, 20% overlap: devrel content","writer",1000],
  ["signal-ingest","general","Listening. 80% x402 ecosystem monitoring, 20% overlap: pattern spotting","writer",1000],
  ["signal-booster","general","Amplification. 80% distribution of reliability reports, 20% overlap: community","writer",1000],
  ["pattern-spotter","researcher","Patterns. 80% trend detection in agent payments, 20% overlap: trend propagation","writer",1000],
  ["trend-propagation","general","Trends. 80% riding ecosystem narratives, 20% overlap: signal boost","writer",1000],
  ["story-collector","general","Stories. 80% seller/agent case studies, 20% overlap: community","writer",1000],
  ["community-weaver","general","Community. 80% x402 builder relationships, 20% overlap: stories","writer",1000],
  ["myth-buster","general","Myth busting. 80% correcting x402 misinformation with data, 20% overlap: claim audit","writer",1000],
  ["counter-narrative","general","Narrative response. 80% responding to criticism with receipts, 20% overlap: myth busting","writer",1000],
  // ---- Analysis (under analyst) ----
  ["analyst","researcher","Head of Analysis. 80% business analysis + metrics, 20% overlap: pricing","orchestrator",2500],
  ["data-scientist","researcher","Data science. 80% trust-score + drift modeling, 20% overlap: analysis","analyst",1000],
  ["researcher","researcher","Research. 80% deep research, 20% overlap: OSINT","analyst",1000],
  ["researcher-hand","researcher","Research hand. 80% retrieval, 20% overlap: browser tasks","analyst",1000],
  ["browser-hand","general","Browser hand. 80% web automation, 20% overlap: research retrieval","analyst",1000],
];

async function ensureCompany(existing) {
  const desc = `${MISSION} ${ANTHEM}`;
  const found = existing.find((c) => c.name === "Code402");
  if (found) {
    console.log(`company exists: Code402 (syncing description/budget)`);
    if (!DRY) await companyUpdate(found.id, { description: desc, budgetMonthlyCents: 50000 });
    return found.id;
  }
  if (DRY) { console.log(`[dry] create company Code402`); return "DRY"; }
  const c = await api("POST", "/companies", { name: "Code402", description: desc, budgetMonthlyCents: 50000 });
  console.log(`created company: Code402 (${c.id})`);
  return c.id;
}

(async () => {
  try { await api("GET", "/health"); }
  catch {
    console.error("Paperclip server not reachable at " + BASE);
    console.error("Start it first:  npx paperclipai run   (dashboard: http://127.0.0.1:3100 — note the :3100 port)");
    process.exit(1);
  }
  const companies = await api("GET", "/companies");
  const cid = await ensureCompany(companies);

  const ids = {};
  let existing = [];
  if (!DRY) existing = await api("GET", `/companies/${cid}/agents`);
  for (const a of existing) ids[a.name] = a.id;

  if (!DRY) await companyUpdate(cid, { requireBoardApprovalForNewAgents: false });
  let hired = 0, skipped = 0;
  for (const [name, role, title, manager, budgetMonthlyCents] of ROSTER) {
    if (ids[name]) { skipped++; continue; }
    const payload = { name, role, title, budgetMonthlyCents };
    if (manager && ids[manager]) payload.reportsTo = ids[manager];
    if (DRY) { console.log(`[dry] ${name} -> ${manager ?? "(top)"}`); ids[name] = "DRY"; hired++; continue; }
    const r = await api("POST", `/companies/${cid}/agents`, payload);
    ids[name] = r.id;
    hired++;
    console.log(`  hired: ${name.padEnd(24)} -> ${manager ?? "(top)"}`);
  }
  if (!DRY) await companyUpdate(cid, { requireBoardApprovalForNewAgents: true });

  console.log(`\nCode402: ${hired} hired, ${skipped} already existed. Total seats: ${ROSTER.length}.`);
  console.log("Org chart: http://127.0.0.1:3100 → Code402 → Org.");
  console.log("Heartbeats are OFF. Next: wire runtimes (OpenFang/OpenClaw/Kimi/Claude), then arm one pod at a time.");
})();
