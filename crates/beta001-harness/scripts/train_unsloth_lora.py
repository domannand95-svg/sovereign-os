#!/usr/bin/env python3
"""
Sovereign OS — Unsloth LoRA Specialization Pipeline

Trains an 8B foundation model (e.g., Llama-3.2-3B or Qwen2.5-7B) on CandidateAgentProposal
schema adherence while tracking complete artifact provenance.

Provenance Tracked:
- base_model_digest
- dataset_canonical_digest
- dataset_tokenized_digest
- adapter_digest
- runtime_artifact_digest
"""

import argparse
import hashlib
import json
from pathlib import Path

def compute_sha256(file_path: Path) -> str:
    h = hashlib.sha256()
    with open(file_path, "rb") as f:
        while chunk := f.read(8192):
            h.update(chunk)
    return h.hexdigest()

def main():
    parser = argparse.ArgumentParser(description="Train Sovereign OS Specialized LoRA Adapter")
    parser.add_argument("--base-model", type=str, default="unsloth/Llama-3.2-3B-Instruct", help="HuggingFace model ID")
    parser.add_argument("--dataset-path", type=str, default="../datasets/train_specialization.jsonl")
    parser.add_argument("--output-dir", type=str, default="../models/sovereign-lora-8b")
    parser.add_argument("--max-seq-length", type=int, default=1024)
    parser.add_argument("--epochs", type=int, default=3)
    parser.add_argument("--batch-size", type=int, default=2)
    parser.add_argument("--gradient-accumulation-steps", type=int, default=4)
    parser.add_argument("--learning-rate", type=float, default=2e-4)
    args = parser.parse_args()

    dataset_file = Path(args.dataset_path).resolve()
    if not dataset_file.exists():
        raise FileNotFoundError(f"Dataset not found at {dataset_file}")

    canonical_dataset_digest = compute_sha256(dataset_file)
    print(f"[*] Canonical Dataset Digest: {canonical_dataset_digest}")

    try:
        from unsloth import FastLanguageModel
        from trl import SFTTrainer
        from transformers import TrainingArguments
        from datasets import load_dataset
    except ImportError:
        print("[!] Unsloth dependencies not installed in current environment.")
        print("[!] Run: pip install unsloth torch transformers datasets trl")
        return

    # 1. Load Base Model in 4-bit
    model, tokenizer = FastLanguageModel.from_pretrained(
        model_name=args.base_model,
        max_seq_length=args.max_seq_length,
        load_in_4bit=True,
    )

    # 2. Add LoRA Adapters
    model = FastLanguageModel.get_peft_model(
        model,
        r=16,
        target_modules=["q_proj", "k_proj", "v_proj", "o_proj", "gate_proj", "up_proj", "down_proj"],
        lora_alpha=16,
        lora_dropout=0,
        bias="none",
        use_gradient_checkpointing="unsloth",
        random_state=3407,
    )

    # 3. Load & Format Dataset
    dataset = load_dataset("json", data_files=str(dataset_file), split="train")

    def formatting_prompts_func(examples):
        convs = examples["conversations"]
        texts = [tokenizer.apply_chat_template(conv, tokenize=False, add_generation_prompt=False) for conv in convs]
        return {"text": texts}

    dataset = dataset.map(formatting_prompts_func, batched=True)

    # 4. Configure Trainer
    trainer = SFTTrainer(
        model=model,
        tokenizer=tokenizer,
        train_dataset=dataset,
        dataset_text_field="text",
        max_seq_length=args.max_seq_length,
        dataset_num_proc=2,
        packing=False,
        args=TrainingArguments(
            per_device_train_batch_size=args.batch_size,
            gradient_accumulation_steps=args.gradient_accumulation_steps,
            warmup_steps=5,
            num_train_epochs=args.epochs,
            learning_rate=args.learning_rate,
            fp16=True,
            logging_steps=10,
            output_dir=args.output_dir,
            seed=3407,
        ),
    )

    print("[*] Starting Unsloth training pass...")
    trainer.train()

    # 5. Export Adapter & GGUF Artifacts
    out_dir = Path(args.output_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    
    model.save_pretrained(str(out_dir / "lora_adapter"))
    tokenizer.save_pretrained(str(out_dir / "lora_adapter"))

    # Export to GGUF for Ollama runtime
    gguf_path = out_dir / "model-q4_k_m.gguf"
    model.save_pretrained_gguf(str(out_dir), tokenizer, quantization_method="q4_k_m")

    # 6. Seal Provenance Ledger
    provenance = {
        "base_model": args.base_model,
        "dataset_canonical_digest": canonical_dataset_digest,
        "adapter_digest": compute_sha256(out_dir / "lora_adapter" / "adapter_model.safetensors") if (out_dir / "lora_adapter" / "adapter_model.safetensors").exists() else "unknown",
        "runtime_artifact_digest": compute_sha256(gguf_path) if gguf_path.exists() else "unknown",
        "export_format": "gguf-q4_k_m"
    }

    (out_dir / "provenance_ledger.json").write_text(json.dumps(provenance, indent=2))
    print(f"[+] Training complete. Provenance ledger written to {out_dir / 'provenance_ledger.json'}")

if __name__ == "__main__":
    main()