import { useEffect, useState } from 'react'
import { LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer } from 'recharts'
import { Activity, ArrowRight, Wifi, TrendingUp, Wallet, History, ShieldCheck, Zap, Info } from 'lucide-react'
import clsx from 'clsx'

interface Trade {
  timestamp: string;
  symbol: string;
  long_ex: string;
  short_ex: string;
  profit_usd: number;
  leverage: number;
}

interface ArbitrageOpportunity {
  symbol: string;
  buy_exchange: string;
  sell_exchange: string;
  buy_price: number;
  sell_price: number;
  max_tradeable_usd: number;
  spread_pct: number;
  total_fees_pct: number;
  net_profit_usd: number;
  liquidity_bottleneck: string;
  timestamp: number;
  data_age_ms: number;
}

interface SimStats {
  total_usd: number;
  binance_usd: number;
  bybit_usd: number;
  hyperliquid_usd: number;
  extended_usd: number;
  trade_count: number;
  last_action: string;
}

interface DashboardPayload {
  opportunities: ArbitrageOpportunity[];
  stats: SimStats;
  last_trades: Trade[];
}

const TradeHistory = ({ trades }: { trades: Trade[] }) => (
  <div className="bg-[#0f0f11] border border-white/5 rounded-[2rem] p-8 shadow-2xl relative overflow-hidden h-[500px] flex flex-col">
    <div className="flex justify-between items-center mb-6 border-b border-white/5 pb-4">
      <h3 className="text-[10px] font-black uppercase tracking-[0.3em] text-gray-400 flex items-center gap-2">
        <History size={14} className="text-primary" /> Live Execution Log
      </h3>
      <span className="flex h-2 w-2 rounded-full bg-green-500 animate-pulse"></span>
    </div>
    
    <div className="space-y-3 overflow-y-auto pr-2 custom-scrollbar flex-1">
      {trades.length === 0 ? (
        <p className="text-[10px] text-gray-600 italic text-center mt-10 uppercase tracking-widest">Waiting for first execution...</p>
      ) : (
        trades.map((trade, i) => (
          <div key={i} className="flex flex-col p-4 bg-white/[0.02] border border-white/5 rounded-2xl hover:bg-white/[0.04] transition-all group">
            <div className="flex justify-between items-start mb-2">
              <span className="font-mono text-[10px] text-gray-500 uppercase">{trade.timestamp}</span>
              <span className="text-[11px] font-black text-green-400 group-hover:scale-110 transition-transform">
                +${trade.profit_usd?.toFixed(4) || "0.0000"}
              </span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-sm font-black italic tracking-tighter text-white">{trade.symbol}</span>
              <div className="flex items-center gap-1.5 text-[9px] font-black text-gray-500">
                {/* CAMBIO AQUÍ: long_ex y short_ex */}
                <span className="text-blue-400">{trade.long_ex}</span> 
                <ArrowRight size={10} />
                <span className="text-purple-400">{trade.short_ex}</span>
              </div>
            </div>
            {/* Opcional: Mostrar el apalancamiento */}
            <div className="text-[8px] text-gray-600 mt-1 uppercase font-bold">
              {trade.leverage}x Leverage
            </div>
          </div>
        ))
      )}
    </div>
  </div>
);

function App() {
  const [opportunities, setOpportunities] = useState<ArbitrageOpportunity[]>([]);
  const [recentTrades, setRecentTrades] = useState<Trade[]>([]);
  const [stats, setStats] = useState<SimStats>({ 
    total_usd: 10000, binance_usd: 2500, bybit_usd: 2500, 
    hyperliquid_usd: 2500, extended_usd: 2500, 
    trade_count: 0, last_action: "Motor en espera..." 
  });
  const [connected, setConnected] = useState(false);
  const [history, setHistory] = useState<{ time: string, balance: number }[]>([]);
  const [frozenOps, setFrozenOps] = useState<ArbitrageOpportunity[]>([]);
  const [mousePos, setMousePos] = useState({ x: 0, y: 0 });

  useEffect(() => {
    const connect = () => {
      const ws = new WebSocket('ws://13.158.22.50:3030/ws');
      ws.onopen = () => setConnected(true);
      ws.onclose = () => { setConnected(false); setTimeout(connect, 3000); };
      ws.onmessage = (event) => {
        try {
          const data: DashboardPayload = JSON.parse(event.data);
          if (data.stats) {
            setStats(data.stats);
            setOpportunities(data.opportunities || []);
            setRecentTrades(data.last_trades || []);
            setHistory(prev => {
              const now = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
              // Solo agregar al historial si el balance cambió o si es el primer punto
              if (prev.length > 0 && prev[prev.length - 1].balance === data.stats.total_usd && prev.length > 1) return prev;
              return [...prev, { time: now, balance: data.stats.total_usd }].slice(-50);
            });
          }
        } catch (e) { console.error("WS Error:", e); }
      };
    };
    connect();
  }, []);

  const handleMouseMove = (e: React.MouseEvent) => {
    setMousePos({ x: e.clientX, y: e.clientY });
  };

  const toggleFreeze = (op: ArbitrageOpportunity) => {
    const isAlreadyFrozen = frozenOps.some(f => f.symbol === op.symbol && f.buy_price === op.buy_price);
    if (isAlreadyFrozen) {
      setFrozenOps(prev => prev.filter(f => !(f.symbol === op.symbol && f.buy_price === op.buy_price)));
    } else {
      setFrozenOps(prev => [op, ...prev]);
    }
  };

  const getOrderAge = (ts: number) => {
    const diff = (Date.now() - ts) / 1000;
    return diff < 0.8 ? "NEW" : `${diff.toFixed(1)}s`;
  };

  const FloatingAnalysis = ({ op }: { op: ArbitrageOpportunity }) => (
    <div 
      className="fixed z-[999] w-72 bg-[#0a0a0c] border border-primary/40 rounded-2xl p-6 shadow-[0_20px_50px_rgba(0,0,0,0.8)] pointer-events-none transition-opacity duration-200"
      style={{ left: mousePos.x + 20, top: mousePos.y + 20 }}
    >
      <div className="flex items-center gap-2 mb-4 border-b border-white/10 pb-2">
        <Zap size={14} className="text-primary" fill="currentColor" />
        <span className="text-[10px] font-black uppercase tracking-widest text-primary">Análisis Técnico</span>
      </div>
      <div className="space-y-4">
        <div>
          <p className="text-[9px] text-gray-500 uppercase font-black tracking-widest">Fixed Node Capital</p>
          <p className="text-base font-bold text-white">$2,500.00 USD</p>
        </div>
        <div>
          <p className="text-[9px] text-gray-500 uppercase font-black tracking-widest">Profit Proyectado</p>
          <p className="text-base font-bold text-green-400">
            +${((2500 / op.buy_price) * op.sell_price - 2500 - (2500 * (op.total_fees_pct/100))).toFixed(4)}
          </p>
        </div>
        <div className="grid grid-cols-2 gap-2 border-t border-white/5 pt-3">
          <div><p className="text-[9px] text-gray-500 uppercase font-black">Feed Age</p><p className="text-xs font-bold text-blue-400">{getOrderAge(op.timestamp)}</p></div>
          <div><p className="text-[9px] text-gray-500 uppercase font-black">Latency</p><p className="text-xs font-bold text-white">{op.data_age_ms}ms</p></div>
        </div>
      </div>
    </div>
  );

  return (
    <div className="min-h-screen p-6 max-w-7xl mx-auto font-mono text-sm bg-[#050505] text-white" onMouseMove={handleMouseMove}>
      {/* HEADER */}
      <header className="flex justify-between items-center mb-6 border-b border-white/5 pb-4">
        <h1 className="text-2xl font-black text-primary flex items-center gap-2 tracking-tighter italic">
          <Activity className="h-6 w-6 text-primary" /> FLASH-ARB <span className="text-white not-italic opacity-40 text-sm tracking-normal">v2.0 PRO</span>
        </h1>
        <div className={clsx("px-4 py-1.5 rounded-full text-[10px] font-black border transition-all tracking-widest", 
          connected ? "bg-green-500/10 text-green-400 border-green-500/30" : "bg-red-500/10 text-red-400 border-red-500/30")}>
          <Wifi className="inline h-3 w-3 mr-2" /> {connected ? "TOKIO NODE: ONLINE" : "OFFLINE"}
        </div>
      </header>

      {/* WALLETS CON ROI */}
      <div className="grid grid-cols-2 md:grid-cols-5 gap-4 mb-6">
        <div className="bg-[#0f0f11] border border-primary/30 p-5 rounded-2xl shadow-xl shadow-primary/5 relative overflow-hidden">
          <div className="absolute -right-2 -top-2 opacity-5"><TrendingUp size={60} /></div>
          <p className="text-[9px] text-primary font-black uppercase tracking-[0.2em] mb-1">Total Equity</p>
          <p className="text-3xl font-black tracking-tighter">${stats.total_usd.toFixed(2)}</p>
          <p className={clsx("text-[10px] font-bold mt-1", stats.total_usd >= 10000 ? "text-green-500" : "text-red-500")}>
            ROI: {(((stats.total_usd - 10000) / 10000) * 100).toFixed(4)}%
          </p>
        </div>
        {['Binance', 'Bybit', 'Hyperliquid', 'Extended'].map((name) => (
          <div key={name} className="bg-[#0f0f11] border border-white/5 p-5 rounded-2xl">
            <p className="text-[9px] text-gray-500 font-bold uppercase tracking-widest mb-1">{name}</p>
            <p className="text-xl font-bold tracking-tight">${(stats as any)[`${name.toLowerCase()}_usd`]?.toFixed(2)}</p>
            <div className="h-1 w-full bg-white/5 mt-3 rounded-full overflow-hidden">
              <div className="h-full bg-primary/40" style={{ width: `${((stats as any)[`${name.toLowerCase()}_usd`] / stats.total_usd) * 100}%` }} />
            </div>
          </div>
        ))}
      </div>

      {/* SYSTEM LOG BAR RE-AGREGADA */}
      <div className="mb-8 px-4 py-2.5 bg-[#0f0f11] border border-white/5 rounded-xl flex justify-between items-center text-[11px] shadow-lg">
        <div className="flex items-center gap-3 text-gray-400">
          <History size={14} className="text-accent" /> 
          <span className="font-black uppercase tracking-widest text-[9px] opacity-50">Last Action:</span> 
          <span className="text-white font-bold uppercase">{stats.last_action}</span>
        </div>
        <div className="flex items-center gap-8">
          <span className="flex items-center gap-1.5 text-green-500 font-black tracking-widest">
            <ShieldCheck size={14}/> VWAP ACTIVE
          </span>
          <span className="text-gray-500 font-black tracking-widest uppercase">
            Trades Executed: <span className="text-white ml-1">{stats.trade_count}</span>
          </span>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-10">
        <div className="lg:col-span-2 space-y-6">
          
          {/* MÚLTIPLES SNAPSHOTS CON DATA COMPLETA */}
          {frozenOps.length > 0 && (
            <div className="space-y-6 mb-12">
               <h2 className="text-[10px] font-black uppercase tracking-[0.5em] text-blue-500 mb-4 flex items-center gap-2">
                 <ShieldCheck size={14} /> Snapshots Bloqueados ({frozenOps.length})
               </h2>
               {frozenOps.map((op, i) => (
                 <div key={`frozen-${i}`} className="bg-blue-500/5 border-2 border-blue-500/40 rounded-[1.5rem] p-8 relative group transition-all hover:bg-blue-500/10 cursor-default overflow-hidden shadow-2xl">
                    <div className="opacity-0 group-hover:opacity-100"><FloatingAnalysis op={op} /></div>
                    <button onClick={() => toggleFreeze(op)} className="absolute top-4 right-6 bg-blue-500 text-white px-3 py-1 rounded-lg text-[10px] font-black z-[110] hover:scale-105 active:scale-95 transition-all">✕ LIBERAR</button>
                    
                    <div className="flex justify-between items-start mb-10 relative z-10">
                      <div className="flex items-center gap-8">
                        <h3 className="text-4xl font-black text-white tracking-tighter italic">{op.symbol}</h3>
                        <div className="h-10 w-[1px] bg-blue-500/20" />
                        <div className="flex flex-col gap-1">
                          <div className="flex items-center gap-3 text-[10px] font-black uppercase tracking-widest text-blue-400">
                            <span>{op.buy_exchange}</span>
                            <ArrowRight size={12} />
                            <span>{op.sell_exchange}</span>
                          </div>
                          <span className="text-[9px] text-gray-500 font-bold uppercase">Static Snapshot</span>
                        </div>
                      </div>
                      <div className="text-right">
                        <p className="text-5xl font-black text-white tracking-tighter leading-none">${op.net_profit_usd.toFixed(4)}</p>
                        <p className="text-[10px] font-black text-gray-600 mt-2 uppercase tracking-widest">Profit Capturado</p>
                      </div>
                    </div>
                    <div className="grid grid-cols-4 gap-12 pt-10 border-t border-blue-500/20 relative z-10">
                      <div className="space-y-2"><span className="text-[10px] text-gray-600 font-black uppercase tracking-widest block">Entry</span><span className="text-xl font-bold text-white tracking-tighter">${op.buy_price.toFixed(4)}</span></div>
                      <div className="space-y-2"><span className="text-[10px] text-gray-600 font-black uppercase tracking-widest block">Exit</span><span className="text-xl font-bold text-white tracking-tighter">${op.sell_price.toFixed(4)}</span></div>
                      <div className="space-y-2"><span className="text-[10px] text-gray-600 font-black uppercase tracking-widest block font-bold">Spread</span><span className="text-xl font-bold text-yellow-500 tracking-tighter">{op.spread_pct.toFixed(3)}%</span></div>
                      <div className="space-y-2 text-right"><span className="text-[10px] text-gray-600 font-black uppercase tracking-widest block">Liquidity</span><span className="text-xl font-bold text-white/90 tracking-tighter">${op.max_tradeable_usd.toLocaleString()}</span></div>
                    </div>
                 </div>
               ))}
            </div>
          )}

          <h2 className="text-[10px] font-black uppercase tracking-[0.5em] text-white/20 mb-6 flex items-center gap-4">
             <div className="h-[1px] w-12 bg-white/10" /> Arbitrage Live Feed
          </h2>

          {opportunities.length === 0 ? (
            <div className="h-80 border border-dashed border-white/5 rounded-[2rem] flex flex-col items-center justify-center text-gray-700 gap-6">
              <div className="animate-spin h-8 w-8 border-2 border-primary border-t-transparent rounded-full" />
              <p className="tracking-[0.3em] text-[10px] uppercase font-black italic">Escaneando Mercado...</p>
            </div>
          ) : (
            opportunities.map((op, i) => (
              <div key={i} onClick={() => toggleFreeze(op)} className="bg-[#0f0f11] border border-white/5 rounded-[1.5rem] p-8 hover:border-primary transition-all group relative cursor-pointer overflow-hidden shadow-lg hover:shadow-primary/5">
                <div className="opacity-0 group-hover:opacity-100"><FloatingAnalysis op={op} /></div>

                <div className="flex justify-between items-start mb-10 relative z-10">
                  <div className="flex items-center gap-8">
                    <h3 className="text-4xl font-black text-white tracking-tighter italic">{op.symbol}</h3>
                    <div className="h-10 w-[1px] bg-white/10" />
                    <div className="flex flex-col gap-1">
                      <div className="flex items-center gap-3 text-[10px] font-black uppercase tracking-widest text-gray-500">
                        <span className="text-blue-400">{op.buy_exchange}</span>
                        <ArrowRight size={12} className="text-gray-700" />
                        <span className="text-purple-400">{op.sell_exchange}</span>
                      </div>
                      <span className="text-[9px] text-blue-500 font-bold uppercase">{getOrderAge(op.timestamp)}</span>
                    </div>
                  </div>
                  <div className="text-right">
                    <p className="text-5xl font-black text-primary tracking-tighter leading-none">+${op.net_profit_usd.toFixed(4)}</p>
                    <p className="text-[10px] font-black text-gray-600 mt-2 uppercase tracking-widest italic leading-none">Est. Net Profit</p>
                  </div>
                </div>

                <div className="grid grid-cols-4 gap-12 pt-10 border-t border-white/5 relative z-10">
                  <div className="space-y-2"><span className="text-[10px] text-gray-600 font-black uppercase tracking-widest block font-bold">Entry</span><span className="text-xl font-bold text-white tracking-tighter">${op.buy_price.toFixed(4)}</span></div>
                  <div className="space-y-2"><span className="text-[10px] text-gray-600 font-black uppercase tracking-widest block font-bold">Exit</span><span className="text-xl font-bold text-white tracking-tighter">${op.sell_price.toFixed(4)}</span></div>
                  <div className="space-y-2"><span className="text-[10px] text-gray-600 font-black uppercase tracking-widest block font-bold">Spread</span><span className="text-xl font-bold text-yellow-500 tracking-tighter">{op.spread_pct.toFixed(3)}%</span></div>
                  <div className="space-y-2 text-right"><span className="text-[10px] text-gray-600 font-black uppercase tracking-widest block font-bold">Volume</span><span className="text-xl font-bold text-white/90 tracking-tighter">${op.max_tradeable_usd.toLocaleString()}</span></div>
                </div>
              </div>
            ))
          )}
        </div>

        {/* SIDEBAR: CHART & ENGINE LOGIC */}
        <div className="space-y-10">          
          {/* 3. Colocamos el nuevo componente aquí */}
          <TradeHistory trades={recentTrades} />
          <div className="bg-[#0f0f11] border border-white/5 rounded-[2rem] p-10 h-96 shadow-2xl">
             <h3 className="text-[10px] font-black text-gray-600 uppercase tracking-[0.3em] mb-10 italic">Portfolio Performance</h3>
             <div className="h-[220px]">
                <ResponsiveContainer width="100%" height="100%">
                  <LineChart data={history}>
                    <Line type="stepAfter" dataKey="balance" stroke="#22c55e" strokeWidth={4} dot={false} isAnimationActive={false} />
                  </LineChart>
                </ResponsiveContainer>
             </div>
             <p className="mt-4 text-[9px] text-gray-500 text-center uppercase tracking-widest italic leading-relaxed">
               La curva se expande automáticamente con cada ejecución detectada.
             </p>
          </div>

          <div className="bg-[#0f0f11] border border-white/5 rounded-[2rem] p-10 shadow-2xl relative overflow-hidden group">
            <div className="flex items-center gap-3 mb-10 border-b border-white/5 pb-6">
               <ShieldCheck className="text-accent" size={20} />
               <h3 className="text-xs font-black uppercase tracking-[0.2em] italic text-white">Engine Logic v2.0</h3>
            </div>
            
            <div className="space-y-10 text-[11px] leading-relaxed">
              <div className="flex gap-6 group">
                 <div className="h-12 w-12 shrink-0 bg-accent/10 rounded-2xl flex items-center justify-center text-accent border border-accent/20 group-hover:bg-accent group-hover:text-black transition-all duration-300">
                    <Wallet size={20} />
                 </div>
                 <div className="space-y-2 pt-1">
                    <p className="text-white font-black uppercase text-[10px] tracking-widest leading-none">Isolated Wallets</p>
                    <p className="text-gray-500 font-medium leading-normal">Balances segregados por nodo ($2.5k c/u). Gestión de riesgo aislada.</p>
                 </div>
              </div>
              <div className="flex gap-6 group">
                 <div className="h-12 w-12 shrink-0 bg-primary/10 rounded-2xl flex items-center justify-center text-primary border border-primary/20 group-hover:bg-primary group-hover:text-black transition-all duration-300">
                    <Zap size={20} />
                 </div>
                 <div className="space-y-2 pt-1">
                    <p className="text-white font-black uppercase text-[10px] tracking-widest leading-none">VWAP Pricing</p>
                    <p className="text-gray-500 font-medium leading-normal">Simulación de impacto en Order Book. Precio dinámico por volumen.</p>
                 </div>
              </div>
              <div className="flex gap-6 group">
                 <div className="h-12 w-12 shrink-0 bg-yellow-500/10 rounded-2xl flex items-center justify-center text-yellow-500 border border-yellow-500/20 group-hover:bg-yellow-500 group-hover:text-black transition-all duration-300">
                    <ShieldCheck size={20} />
                 </div>
                 <div className="space-y-2 pt-1">
                    <p className="text-white font-black uppercase text-[10px] tracking-widest leading-none">Anti-Latency</p>
                    <p className="text-gray-500 font-medium leading-normal">Solo se procesan datos bajo el umbral de ejecución configurado.</p>
                 </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;