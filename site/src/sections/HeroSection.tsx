import { useState, useEffect } from "react";
import { ArrowRight, Shield, Zap, Globe, Lock } from "lucide-react";
import { Button } from "@/components/ui/button";

export function HeroSection() {
  const [count, setCount] = useState(0);
  const target = 75410000; // 75.41M transactions

  useEffect(() => {
    const duration = 2000;
    const steps = 60;
    const increment = target / steps;
    let current = 0;
    const timer = setInterval(() => {
      current += increment;
      if (current >= target) {
        setCount(target);
        clearInterval(timer);
      } else {
        setCount(Math.floor(current));
      }
    }, duration / steps);
    return () => clearInterval(timer);
  }, []);

  return (
    <section className="relative min-h-screen flex items-center justify-center overflow-hidden bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950 text-white">
      {/* Animated grid background */}
      <div className="absolute inset-0 opacity-10">
        <div 
          className="absolute inset-0"
          style={{
            backgroundImage: `linear-gradient(rgba(255,255,255,0.03) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.03) 1px, transparent 1px)`,
            backgroundSize: '50px 50px'
          }}
        />
      </div>
      
      {/* Glow effects */}
      <div className="absolute top-1/4 left-1/4 w-96 h-96 bg-orange-500/10 rounded-full blur-3xl" />
      <div className="absolute bottom-1/4 right-1/4 w-96 h-96 bg-blue-500/10 rounded-full blur-3xl" />

      <div className="relative z-10 max-w-5xl mx-auto px-6 text-center">
        <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-orange-500/10 border border-orange-500/20 text-orange-400 text-sm font-mono mb-8">
          <Zap className="w-4 h-4" />
          <span>HTTP 402 Payment Required — Now Usable</span>
        </div>

        <h1 className="text-5xl md:text-7xl font-bold tracking-tight mb-6">
          <span className="bg-gradient-to-r from-orange-400 via-amber-300 to-orange-400 bg-clip-text text-transparent">
            x402
          </span>
        </h1>
        
        <p className="text-xl md:text-2xl text-slate-400 mb-4 font-light">
          Internet-Native Payments for Agents, APIs, and Assets
        </p>
        
        <p className="text-lg text-slate-500 max-w-2xl mx-auto mb-10">
          The open standard that finally puts HTTP 402 to use. 
          Machine-to-machine payments with zero accounts, zero sessions, zero API keys.
        </p>

        <div className="flex flex-col sm:flex-row gap-4 justify-center mb-16">
          <Button 
            size="lg" 
            className="bg-orange-500 hover:bg-orange-600 text-white px-8"
            onClick={() => document.getElementById('demo')?.scrollIntoView({ behavior: 'smooth' })}
          >
            Try Interactive Demo <ArrowRight className="ml-2 w-4 h-4" />
          </Button>
          <Button 
            size="lg" 
            variant="outline" 
            className="border-slate-700 text-slate-300 hover:bg-slate-800"
            onClick={() => document.getElementById('spec')?.scrollIntoView({ behavior: 'smooth' })}
          >
            Read the Spec
          </Button>
        </div>

        {/* Stats */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-8 max-w-3xl mx-auto">
          <div className="p-6 rounded-2xl bg-slate-900/50 border border-slate-800">
            <div className="text-3xl font-bold text-orange-400 font-mono">
              {(count / 1000000).toFixed(2)}M
            </div>
            <div className="text-sm text-slate-500 mt-1">Transactions (30d)</div>
          </div>
          <div className="p-6 rounded-2xl bg-slate-900/50 border border-slate-800">
            <div className="text-3xl font-bold text-green-400 font-mono">$24.24M</div>
            <div className="text-sm text-slate-500 mt-1">Volume (30d)</div>
          </div>
          <div className="p-6 rounded-2xl bg-slate-900/50 border border-slate-800">
            <div className="text-3xl font-bold text-blue-400 font-mono">22K+</div>
            <div className="text-sm text-slate-500 mt-1">Active Sellers</div>
          </div>
        </div>

        {/* Feature pills */}
        <div className="flex flex-wrap justify-center gap-4 mt-12">
          {[
            { icon: Shield, label: "Trust-Minimized" },
            { icon: Globe, label: "Rail Agnostic" },
            { icon: Lock, label: "No Accounts" },
            { icon: Zap, label: "Sub-Second" },
          ].map(({ icon: Icon, label }) => (
            <div key={label} className="flex items-center gap-2 px-4 py-2 rounded-full bg-slate-800/50 border border-slate-700 text-slate-400 text-sm">
              <Icon className="w-4 h-4" />
              {label}
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
