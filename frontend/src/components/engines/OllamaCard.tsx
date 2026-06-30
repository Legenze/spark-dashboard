import { MetricTile } from './EngineCardPrimitives'
import type { EngineSnapshot } from '@/types/metrics'

interface OllamaCardProps {
  engine: EngineSnapshot
}

/**
 * Card for an Ollama engine. Ollama exposes no Prometheus `/metrics` endpoint,
 * so there are no live throughput/latency signals to chart. This card mirrors
 * the visual language of {@link EngineCard} — the same `bg-white/[0.02]` metric
 * boxes and {@link MetricTile} primitives — but surfaces only the model identity
 * Ollama reports via `/api/ps`, plus a note explaining why performance metrics
 * are absent.
 */
export function OllamaCard({ engine }: OllamaCardProps) {
  const model = engine.model

  if (model === null) {
    return (
      <div className="flex flex-col py-1">
        <p className="text-sm text-zinc-500">
          No model loaded. Run a model in Ollama (e.g.{' '}
          <span className="font-mono text-zinc-400">ollama run llama3</span>) and it will appear
          here within seconds.
        </p>
      </div>
    )
  }

  const val = (s: string | null) => s ?? '--'

  return (
    <div className="flex flex-col gap-2 py-1">
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-2">
        {/* Model identity */}
        <div className="bg-white/[0.02] rounded-md px-3 py-2.5 2xl:px-4 2xl:py-3 min-w-0">
          <div className="text-[11px] 2xl:text-xs min-[1920px]:text-sm font-semibold text-zinc-300 tracking-tight mb-1.5 truncate">
            Model
          </div>
          <div className="grid grid-cols-3 gap-1.5">
            <MetricTile label="Parameters" value={val(model.parameter_size)} />
            <MetricTile label="Quantization" value={val(model.quantization)} />
            <MetricTile label="Architecture" value={val(model.model_type)} />
          </div>
        </div>

        {/* Runtime */}
        <div className="bg-white/[0.02] rounded-md px-3 py-2.5 2xl:px-4 2xl:py-3 min-w-0">
          <div className="text-[11px] 2xl:text-xs min-[1920px]:text-sm font-semibold text-zinc-300 tracking-tight mb-1.5 truncate">
            Runtime
          </div>
          <div className="flex flex-col gap-0.5 min-w-0">
            <span className="text-[10px] 2xl:text-xs min-[1920px]:text-sm font-medium uppercase tracking-wider truncate text-zinc-400">
              Endpoint
            </span>
            <span className="text-sm 2xl:text-base font-mono text-zinc-200 truncate" title={engine.endpoint}>
              {engine.endpoint}
            </span>
          </div>
        </div>
      </div>

      <p className="text-xs text-zinc-500">
        Ollama does not expose a Prometheus metrics endpoint, so live throughput, latency, and
        cache statistics are unavailable.
      </p>
    </div>
  )
}
