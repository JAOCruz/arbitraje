import { useEffect, useState } from 'react'
import { LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer } from 'recharts'
import { Activity, DollarSign, ArrowRight, AlertTriangle, Wifi, TrendingUp, Wallet, Zap, Trophy, History } from 'lucide-react'
import clsx from 'clsx'

// Interfaces actualizadas según el nuevo Backend Rust
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
}

interface SimStats {
  balance: number;
  total_profit: number;
  trade_count: number;
  last_trade: string;
}

interface DashboardPayload {
  opportunities: ArbitrageOpportunity[];
  stats: SimStats;
}

function App() {
  const [opportunities, setOpportunities] = useState<ArbitrageOpportunity[]>([]);
  const [stats, setStats] = useState<SimStats>({ balance: 10000, total_profit: 0, trade_count: 0, last_trade: "Esperando..." });
  const [investment, setInvestment] = useState<number>(1000);
  const [connected, setConnected] = useState(false);
  const [history, setHistory] = useState<{ time: string, balance: number }[]>([]);

  useEffect(() => {
    const ws = new WebSocket('ws://127.0.0.1:3030/ws');

    ws.onopen = () => setConnected(true);
    ws.onclose = () => setConnected(false);

    ws.onmessage = (event) => {
      try {
        // Parseamos el nuevo payload completo
        const data: DashboardPayload = JSON.parse(event.data);
        
        // 1. Actualizar Oportunidades
        const sorted = data.opportunities.sort((a, b) => b.net_profit_usd - a.net_profit_usd);
        setOpportunities(sorted);

        // 2. Actualizar Stats de Paper Trading
        setStats(data.stats);

        // 3. Historial del Balance (Para el gráfico)
        setHistory(prev => {
          const now = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second:'2-digit' });
          // Solo añadimos punto si cambió el balance o cada cierto tiempo
          if (prev.length > 0 && prev[prev.length - 1].balance === data.stats.balance && Math.random() > 0.1) return prev;
          
          return [...prev, { time: now, balance: data.stats.balance }].slice(-30);
        });

      } catch (e) {
        console.error("Error parsing WS data", e);
      }
    };

    return () => ws.close();
  }, []);

  return (
    <div className="min-h-screen p-6 max-w-7xl mx-auto font-mono text-sm">
      {/* HEADER */}
      <header className="flex flex-col md:flex-row justify-between items-center mb-8 border-b border-border pb-4 gap-4">
        <div>
          <h1 className="text-3xl font-black flex items-center gap-3 text-primary tracking-tighter italic">
            <Activity className="h-8 w-8" />
            FLASH-ARB <span className="text-white not-italic text-lg font-normal opacity-50">PRO TERMINAL</span>
          </h1>
        </div>
        <div className="flex items-center gap-4">
           {/* Indicador de Conexión */}
          <div className={clsx("flex items-center gap-2 px-3 py-1 rounded-full text-xs font-bold transition-all", connected ? "bg-green-900/30 text-green-400 border border-green-900" : "bg-red-900/30 text-red-400 border border-red-900")}>
            <Wifi className="h-3 w-3" />
            {connected ? "LIVE FEED" : "OFFLINE"}
          </div>
        </div>
      </header>

      {/* PAPER TRADING STATUS BAR */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-8">
          <div className="bg-surface border border-border rounded-xl p-4 flex flex-col justify-center relative overflow-hidden group">
             <div className="absolute top-0 right-0 p-2 opacity-10 group-hover:opacity-20 transition-opacity"><Wallet size={40} /></div>
             <span className="text-gray-400 text-xs uppercase font-bold">Balance Simulado</span>
             <span className="text-2xl font-bold text-white">${stats.balance.toFixed(2)}</span>
             <div className="text-xs text-green-500 font-bold">Inicial: $10,000.00</div>
          </div>
          
          <div className="bg-surface border border-border rounded-xl p-4 flex flex-col justify-center relative overflow-hidden group">
             <div className="absolute top-0 right-0 p-2 opacity-10 group-hover:opacity-20 transition-opacity"><Trophy size={40} /></div>
             <span className="text-gray-400 text-xs uppercase font-bold">Ganancia Total</span>
             <span className={clsx("text-2xl font-bold", stats.total_profit >= 0 ? "text-primary" : "text-danger")}>
                {stats.total_profit >= 0 ? "+" : ""}{stats.total_profit.toFixed(2)} USD
             </span>
          </div>

          <div className="bg-surface border border-border rounded-xl p-4 flex flex-col justify-center relative overflow-hidden group">
             <div className="absolute top-0 right-0 p-2 opacity-10 group-hover:opacity-20 transition-opacity"><Zap size={40} /></div>
             <span className="text-gray-400 text-xs uppercase font-bold">Trades Ejecutados</span>
             <span className="text-2xl font-bold text-white">{stats.trade_count}</span>
             <div className="text-xs text-blue-400 font-bold">Auto-Execution: ON</div>
          </div>

          <div className="bg-surface border border-border rounded-xl p-4 flex flex-col justify-center relative overflow-hidden group">
             <div className="absolute top-0 right-0 p-2 opacity-10 group-hover:opacity-20 transition-opacity"><History size={40} /></div>
             <span className="text-gray-400 text-xs uppercase font-bold">Última Acción</span>
             <span className="text-sm font-bold text-white truncate" title={stats.last_trade}>{stats.last_trade}</span>
          </div>
      </div>

      {/* GRID PRINCIPAL */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        
        {/* COLUMNA IZQUIERDA: Oportunidades */}
        <div className="lg:col-span-2 space-y-4">
          <h2 className="text-lg font-bold flex items-center gap-2 mb-4 text-white uppercase tracking-wider">
            <DollarSign className="text-primary h-5 w-5" /> Mercado en Tiempo Real
          </h2>
          
          {opportunities.length === 0 ? (
            <div className="p-12 text-center border border-dashed border-border rounded-xl text-gray-500 bg-surface/50">
              Escaneando exchanges...
            </div>
          ) : (
            opportunities.map((op, idx) => {
              const realInvestment = Math.min(investment, op.max_tradeable_usd);
              const grossProfit = realInvestment * (op.spread_pct / 100);
              const fees = realInvestment * (op.total_fees_pct / 100);
              const netProfit = grossProfit - fees;
              const roi = (netProfit / realInvestment) * 100;

              return (
                <div key={idx} className="bg-surface border border-border rounded-xl p-5 hover:border-primary transition-all duration-300 relative overflow-hidden shadow-lg group">
                   {netProfit > 0 && <div className="absolute left-0 top-0 w-1 h-full bg-primary" />}
                  
                  <div className="flex justify-between items-start relative z-10">
                    <div>
                      <h3 className="text-2xl font-black text-white flex items-center gap-2 tracking-tight">
                        {op.symbol}
                      </h3>
                      <div className="flex items-center gap-2 mt-2 text-xs font-bold uppercase tracking-wider">
                        <span className="text-blue-400">{op.buy_exchange}</span>
                        <ArrowRight className="h-3 w-3 text-gray-600" />
                        <span className="text-purple-400">{op.sell_exchange}</span>
                      </div>
                    </div>

                    <div className="text-right">
                      <div className={clsx("text-3xl font-black tracking-tighter", netProfit > 0 ? "text-primary" : "text-danger")}>
                        {netProfit > 0 ? "+" : ""}{netProfit.toFixed(4)} <span className="text-sm text-gray-500 font-normal">USD</span>
                      </div>
                      <div className={clsx("text-xs font-bold mt-1", netProfit > 0 ? "text-green-700" : "text-red-800")}>
                        ROI ESTIMADO: {roi.toFixed(3)}%
                      </div>
                    </div>
                  </div>

                  <div className="grid grid-cols-4 gap-4 mt-4 pt-4 border-t border-border/50 text-xs text-gray-400">
                    <div>PRECIO COMPRA: <span className="text-white">${op.buy_price.toFixed(4)}</span></div>
                    <div>PRECIO VENTA: <span className="text-white">${op.sell_price.toFixed(4)}</span></div>
                    <div>SPREAD: <span className="text-yellow-500">{op.spread_pct.toFixed(2)}%</span></div>
                    <div>LIQUIDEZ: <span className="text-white">${op.max_tradeable_usd.toFixed(0)}</span></div>
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* COLUMNA DERECHA: Gráfico de Balance */}
        <div className="space-y-6">
          <div className="bg-surface border border-border rounded-xl p-5 h-80 flex flex-col shadow-lg">
             <h3 className="text-xs font-bold text-gray-400 mb-4 flex items-center gap-2 uppercase tracking-widest">
                <TrendingUp className="h-4 w-4" /> Crecimiento del Portafolio
             </h3>
             <div className="flex-1 w-full min-h-0">
               <ResponsiveContainer width="100%" height="100%">
                 <LineChart data={history}>
                   <Tooltip 
                     contentStyle={{ backgroundColor: '#18181b', borderColor: '#27272a' }}
                     itemStyle={{ color: '#22c55e' }}
                   />
                   <XAxis dataKey="time" hide />
                   <YAxis domain={['auto', 'auto']} stroke="#444" tick={{fontSize: 10}} />
                   <Line type="stepAfter" dataKey="balance" stroke="#22c55e" strokeWidth={2} dot={false} />
                 </LineChart>
               </ResponsiveContainer>
             </div>
          </div>
          
          <div className="p-4 bg-blue-900/10 border border-blue-900/30 rounded-xl text-xs text-blue-300">
             <strong>⚠️ MODO SIMULACIÓN ACTIVO</strong>
             <p className="mt-1 opacity-70">El bot está "operando" con una billetera virtual de $10,000. Los trades se registran automáticamente cuando el ROI supera el 0.01%.</p>
          </div>
        </div>
      </div>
    </div>
  )
}

export default App