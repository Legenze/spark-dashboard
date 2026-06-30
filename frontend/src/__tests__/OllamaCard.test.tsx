import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { OllamaCard } from '../components/engines/OllamaCard'
import type { ModelInfo, EngineSnapshot } from '../types/metrics'

function snapshot(model: ModelInfo | null): EngineSnapshot {
  return {
    engine_type: 'Ollama',
    endpoint: 'http://localhost:11434',
    status: { type: 'Running' },
    model,
    // Ollama has no Prometheus metrics, so the backend always sends null here.
    metrics: null,
    recent_requests: [],
    deployment_mode: 'Native',
  }
}

function model(overrides: Partial<ModelInfo> = {}): ModelInfo {
  return {
    name: 'qwen3.5:35b-a3b-q8_0',
    parameter_size: '36.0B',
    quantization: 'Q8_0',
    precision: null,
    tensor_type: null,
    model_type: 'qwen35moe',
    pipeline_tag: null,
    ...overrides,
  }
}

describe('OllamaCard', () => {
  it('renders the model identity Ollama reports', () => {
    render(<OllamaCard engine={snapshot(model())} />)
    expect(screen.getByText('36.0B')).toBeTruthy()
    expect(screen.getByText('Q8_0')).toBeTruthy()
    expect(screen.getByText('qwen35moe')).toBeTruthy()
    expect(screen.getByText('http://localhost:11434')).toBeTruthy()
  })

  it('always explains why live performance metrics are absent', () => {
    render(<OllamaCard engine={snapshot(model())} />)
    expect(screen.getByText(/does not expose a Prometheus metrics endpoint/i)).toBeTruthy()
  })

  it('shows dashes for attributes Ollama did not report', () => {
    render(
      <OllamaCard engine={snapshot(model({ parameter_size: null, model_type: null }))} />,
    )
    // Quantization still present; the two missing fields collapse to '--'.
    expect(screen.getByText('Q8_0')).toBeTruthy()
    expect(screen.getAllByText('--').length).toBe(2)
  })

  it('shows an idle prompt when no model is loaded', () => {
    render(<OllamaCard engine={snapshot(null)} />)
    expect(screen.getByText(/No model loaded/i)).toBeTruthy()
    expect(screen.queryByText(/Prometheus metrics endpoint/i)).toBeNull()
  })
})
