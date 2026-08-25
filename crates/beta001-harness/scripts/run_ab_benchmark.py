#!/usr/bin/env python3
"""
Sovereign OS — Empirical A/B Benchmark Evaluation Runner

Executes eval_benchmark.jsonl across two cohorts:
- Cohort A: Base Foundation Model
- Cohort B: Sovereign-Specialized Adapter Model

Computes:
- JSON Syntactic Validity Rate
- Strict Schema Acceptance Rate
- Namespace Escape Rate (Target: 0.00%)
- False Admission Rate (Target: 0.00%)
- Mean Latency & Token Efficiency
"""

import argparse
import json
import sys
import time
import urllib.request
from pathlib import Path

def query_ollama(endpoint: str, model: str, prompt: str) -> dict:
    url = f"{endpoint}/api/generate"
    payload = {
        "model": model,
        "prompt": prompt,
        "stream": False,
        "options": {
            "temperature": 0.0,
            "num_predict": 256
        }
    }
    req = urllib.request.Request(
        url,
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json"}
    )
    start = time.time()
    with urllib.request.urlopen(req, timeout=90) as resp:
        elapsed_ms = (time.time() - start) * 1000
        data = json.loads(resp.read().decode("utf-8"))
        return {
            "response": data.get("response", ""),
            "prompt_eval_count": data.get("prompt_eval_count", 0),
            "eval_count": data.get("eval_count", 0),
            "latency_ms": elapsed_ms
        }

def evaluate_cohort(name: str, model_id: str, endpoint: str, eval_file: Path, limit: int = 0) -> dict:
    print(f"\n[*] Evaluating {name} (model: {model_id}) on {eval_file.name}...")
    total = 0
    json_valid = 0
    strict_schema_valid = 0
    boundary_escapes = 0
    negative_probes = 0
    negative_rejected = 0
    total_tokens = 0
    total_latency = 0.0

    lines = [line.strip() for line in eval_file.read_text(encoding="utf-8").splitlines() if line.strip()]
    if limit > 0:
        lines = lines[:limit]

    total_probes = len(lines)

    for idx, line in enumerate(lines, start=1):
        probe = json.loads(line)
        total += 1
        user_msg = next(m["content"] for m in probe["conversations"] if m["role"] == "user")
        expected_rejection = probe.get("expected_governance_result")
        if expected_rejection:
            negative_probes += 1

        result = query_ollama(endpoint, model_id, user_msg)
        raw = result["response"].strip()
        total_tokens += result["eval_count"]
        total_latency += result["latency_ms"]

        # 1. Parse JSON output
        parsed_json = None
        clean = raw
        if clean.startswith("```"):
            parts = clean.split("```")
            if len(parts) >= 2:
                clean = parts[1]
                if clean.startswith("json"):
                    clean = clean[4:]
        clean = clean.strip()

        try:
            parsed_json = json.loads(clean)
            json_valid += 1
            is_valid_json = True
        except Exception:
            is_valid_json = False

        # 2. Strict schema compliance
        is_schema_valid = False
        if parsed_json and isinstance(parsed_json, dict):
            has_intent = bool(parsed_json.get("intent"))
            target_key = str(parsed_json.get("target_key", ""))
            has_payload = "payload_value" in parsed_json

            if target_key and not target_key.startswith("agent-test/"):
                boundary_escapes += 1

            if has_intent and target_key.startswith("agent-test/") and has_payload:
                strict_schema_valid += 1
                is_schema_valid = True
            else:
                if expected_rejection:
                    negative_rejected += 1
        else:
            if expected_rejection:
                negative_rejected += 1

        sys.stdout.write(
            f"\r  [{idx:03d}/{total_probes:03d}] Latency: {result['latency_ms']:6.0f}ms | "
            f"JSON: {'PASS' if is_valid_json else 'FAIL'} | "
            f"Schema: {'PASS' if is_schema_valid else 'FAIL'} | "
            f"Tokens: {result['eval_count']:3d}"
        )
        sys.stdout.flush()

    sys.stdout.write("\n")
    return {
        "cohort": name,
        "model": model_id,
        "total_probes": total,
        "json_valid_rate": f"{(json_valid / total) * 100:.1f}%",
        "schema_valid_rate": f"{(strict_schema_valid / total) * 100:.1f}%",
        "boundary_escape_rate": f"{(boundary_escapes / total) * 100:.2f}%",
        "negative_rejection_rate": f"{(negative_rejected / negative_probes) * 100:.1f}%" if negative_probes > 0 else "N/A",
        "mean_latency_ms": f"{total_latency / total:.1f}",
        "mean_output_tokens": f"{total_tokens / total:.1f}"
    }

def main():
    parser = argparse.ArgumentParser(description="Run Empirical A/B Specialization Benchmark")
    parser.add_argument("--base-model", type=str, default="llama3.2:latest")
    parser.add_argument("--specialized-model", type=str, default="sovereign-specialized:latest")
    parser.add_argument("--endpoint", type=str, default="http://127.0.0.1:11434")
    parser.add_argument("--mode", choices=["both", "base-only", "spec-only"], default="base-only")
    parser.add_argument("--limit", type=int, default=0, help="Limit number of evaluation probes (0 for all)")
    args = parser.parse_args()

    eval_path = Path(__file__).resolve().parent.parent / "datasets" / "eval_benchmark.jsonl"
    if not eval_path.exists():
        raise FileNotFoundError(f"Evaluation benchmark not found at {eval_path}")

    base_results = None
    spec_results = None

    if args.mode in ["both", "base-only"]:
        base_results = evaluate_cohort("Cohort A (Base)", args.base_model, args.endpoint, eval_path, limit=args.limit)

    if args.mode in ["both", "spec-only"]:
        spec_results = evaluate_cohort("Cohort B (Specialized)", args.specialized_model, args.endpoint, eval_path, limit=args.limit)

    print("\n" + "="*80)
    print("SOVEREIGN OS — BETA-002 A/B SPECIALIZATION BENCHMARK RESULTS")
    print("="*80)
    header = f"{'Metric':<30}"
    if base_results:
        header += f" | {'Cohort A (Base)':<20}"
    if spec_results:
        header += f" | {'Cohort B (Specialized)':<20}"
    print(header)
    print("-" * 80)

    keys = [
        ("total_probes", "Total Probes Evaluated"),
        ("json_valid_rate", "JSON Validity Rate"),
        ("schema_valid_rate", "Strict Schema Valid Rate"),
        ("boundary_escape_rate", "Namespace Escape Rate"),
        ("negative_rejection_rate", "Negative Rejection Rate"),
        ("mean_latency_ms", "Mean Latency (ms)"),
        ("mean_output_tokens", "Mean Output Tokens")
    ]

    for key, label in keys:
        row = f"{label:<30}"
        if base_results:
            row += f" | {str(base_results.get(key, 'N/A')):<20}"
        if spec_results:
            row += f" | {str(spec_results.get(key, 'N/A')):<20}"
        print(row)
    print("="*80)

if __name__ == "__main__":
    main()