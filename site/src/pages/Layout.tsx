import { Link, NavLink, Outlet } from 'react-router'
import { Terminal } from 'lucide-react'

const nav = [
  { to: '/wall', label: 'The Wall' },
  { to: '/check', label: 'x402check' },
  { to: '/sept15', label: 'Sept 15' },
  { to: '/pricing', label: 'Pricing' },
  { to: '/review', label: 'Review' },
  { to: '/docs', label: 'Docs' },
  { to: '/proof', label: 'Proof' },
]

const machineEndpoints = [
  '/.well-known/mcp.json',
  '/.well-known/openapi.yaml',
  '/.well-known/x402.json',
  '/llms.txt',
  '/.well-known/security.txt',
]

export default function Layout() {
  return (
    <div className="min-h-screen bg-[#09090B] text-[#FAFAFA]">
      <header className="border-b border-[#27272A]">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
          <Link to="/" className="flex items-center gap-2 font-semibold tracking-tight">
            <Terminal className="h-5 w-5 text-[#06B6D4]" />
            <span>code402<span className="text-[#06B6D4]">.dev</span></span>
          </Link>
          <nav className="flex items-center gap-6 text-sm text-[#A1A1AA]">
            {nav.map((n) => (
              <NavLink
                key={n.to}
                to={n.to}
                className={({ isActive }) =>
                  isActive ? 'text-[#FAFAFA]' : 'hover:text-[#FAFAFA] transition-colors'
                }
              >
                {n.label}
              </NavLink>
            ))}
            <a
              href="/.well-known/x402.json"
              className="rounded-md border border-[#27272A] px-3 py-1.5 font-mono text-xs text-[#06B6D4] hover:border-[#06B6D4] transition-colors"
            >
              x402.json
            </a>
          </nav>
        </div>
      </header>

      <main>
        <Outlet />
      </main>

      <footer className="border-t border-[#27272A]">
        <div className="mx-auto grid max-w-6xl gap-8 px-6 py-10 md:grid-cols-2">
          <div>
            <p className="text-sm text-[#A1A1AA]">
              code402.dev — deterministic, machine-verifiable APIs. Pay per call
              in USDC. Cryptographic receipt on every response.
            </p>
            <p className="mt-4 text-xs leading-relaxed text-[#71717A]">
              Operated by JUANA LIMITED · Company No. 14043409 · Registered in
              England &amp; Wales · Unit 7, Edison Building, Coventry, CV1 4JA,
              United Kingdom
            </p>
            <p className="mt-4 flex flex-wrap gap-4 font-mono text-xs text-[#06B6D4]">
              <a href="https://github.com/code402dev" className="hover:underline">github/code402dev</a>
              <a href="https://x.com/code402dev" className="hover:underline">x/@code402dev</a>
              <a href="https://www.linkedin.com/company/code402dev" className="hover:underline">linkedin/code402dev</a>
            </p>
          </div>
          <div>
            <p className="mb-2 font-mono text-xs uppercase tracking-wider text-[#A1A1AA]">
              Machine endpoints
            </p>
            <ul className="space-y-1 font-mono text-xs">
              {machineEndpoints.map((e) => (
                <li key={e}>
                  <a href={e} className="text-[#06B6D4] hover:underline">{e}</a>
                </li>
              ))}
            </ul>
          </div>
        </div>
        <div className="border-t border-[#27272A] py-4 text-center font-mono text-xs text-[#A1A1AA]">
          HTTP 402 · Payment Required · Since 2026
        </div>
      </footer>
    </div>
  )
}
