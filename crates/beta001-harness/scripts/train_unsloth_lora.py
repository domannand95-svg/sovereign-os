#!/usr/bin/env python3
"""
Sovereign OS — CandidateAgentProposal Specialization Training Pipeline

Trains a foundation model (e.g., Llama-3.2-3B-Instruct) using 4-bit QLoRA
for strict CandidateAgentProposal schema adherence while tracking complete artifact provenance.

Provenance Tracked:
- base_model_id
- dataset_canonical_digest
- adapter_digest
- training_config
"""

import argparse
import hashlib
import json
from pathlib import Path

# --- PyTorch Sub-Byte Type Compatibility Shim for Windows ---
import torch

for sub_byte_attr in [
    "int1",
    "int2",
    "int3",
    "int4",
    "int5",
    "int6",
    "int7",
    "uint1",
    "uint2",
    "uint3",
    "uint4",
    "uint5",
    "uint6",
    "uint7",
]:
    if not hasattr(torch, sub_byte_attr):
        setattr(torch, sub_byte_attr, torch.uint8)

def compute_sha256(file_path: Path) -> str:
    h = hashlib.sha256()
    with open(file_path, "rb") as f:
        while chunk := f.read(8192):
            h.update(chunk)
    return h.hexdigest()

def main():
    repo_root = Path(__file__).resolve().parents[3]
    parser = argparse.ArgumentParser(description="Train Sovereign OS Specialized LoRA Adapter")
    parser.add_argument(
        "--base-model",
        type=str,
        default="unsloth/Llama-3.2-3B-Instruct",
        help="Hugging Face model ID",
    )
    parser.add_argument(
        "--dataset-path",
        type=str,
        default=str(
            repo_root
            / "crates"
            / "beta001-harness"
            / "datasets"
            / "train_specialization.jsonl"
        ),
    )
    parser.add_argument(
        "--output-dir",
        type=str,
        default=str(repo_root / "models" / "sovereign-lora-8b"),
    )
    parser.add_argument("--max-seq-length", type=int, default=512)
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

    out_dir = Path(args.output_dir).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    lora_dir = out_dir / "lora_adapter"
    lora_dir.mkdir(parents=True, exist_ok=True)

    from datasets import load_dataset
    from transformers import (
        AutoModelForCausalLM,
        AutoTokenizer,
        BitsAndBytesConfig,
        TrainingArguments,
    )
    from peft import LoraConfig, get_peft_model, prepare_model_for_kbit_training
    from trl import SFTTrainer

    # 1. 4-bit Quantization Configuration
    bnb_config = BitsAndBytesConfig(
        load_in_4bit=True,
        bnb_4bit_quant_type="nf4",
        bnb_4bit_compute_dtype=torch.float16,
        bnb_4bit_use_double_quant=True,
    )

    print(f"[*] Loading base model: {args.base_model}...")
    tokenizer = AutoTokenizer.from_pretrained(args.base_model, trust_remote_code=True)
    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token

    model = AutoModelForCausalLM.from_pretrained(
        args.base_model,
        quantization_config=bnb_config,
        device_map="auto",
        trust_remote_code=True,
    )
    model = prepare_model_for_kbit_training(model)

    # 2. Attach PEFT LoRA Config
    peft_config = LoraConfig(
        r=16,
        lora_alpha=32,
        target_modules=["q_proj", "k_proj", "v_proj", "o_proj", "gate_proj", "up_proj", "down_proj"],
        lora_dropout=0.05,
        bias="none",
        task_type="CAUSAL_LM",
    )
    model = get_peft_model(model, peft_config)
    model.print_trainable_parameters()

    # 3. Format Dataset with Chat Template
    dataset = load_dataset("json", data_files=str(dataset_file), split="train")

    def format_prompts(batch):
        formatted = []
        for convs in batch["conversations"]:
            formatted.append(
                tokenizer.apply_chat_template(
                    convs, tokenize=False, add_generation_prompt=False
                )
            )
        return {"text": formatted}

    dataset = dataset.map(format_prompts, batched=True)

    # 4. Training Arguments
    training_args = TrainingArguments(
        output_dir=str(out_dir / "checkpoints"),
        per_device_train_batch_size=args.batch_size,
        gradient_accumulation_steps=args.gradient_accumulation_steps,
        learning_rate=args.learning_rate,
        num_train_epochs=args.epochs,
        logging_steps=10,
        fp16=True,
        optim="adamw_bnb_8bit",
        save_strategy="no",
        report_to="none",
        seed=3407,
    )

    trainer = SFTTrainer(
        model=model,
        train_dataset=dataset,
        dataset_text_field="text",
        max_seq_length=args.max_seq_length,
        tokenizer=tokenizer,
        args=training_args,
    )

    print("[*] Beginning specialization training...")
    trainer.train()

    # 5. Save Adapter and Tokenizer
    print(f"[*] Saving adapter to {lora_dir}...")
    model.save_pretrained(str(lora_dir))
    tokenizer.save_pretrained(str(lora_dir))

    # 6. Seal Provenance Ledger
    adapter_file = lora_dir / "adapter_model.safetensors"
    adapter_digest = compute_sha256(adapter_file) if adapter_file.exists() else "not_found"

    provenance = {
        "base_model_id": args.base_model,
        "dataset_canonical_digest": canonical_dataset_digest,
        "adapter_digest": adapter_digest,
        "training_hyperparameters": {
            "epochs": args.epochs,
            "batch_size": args.batch_size,
            "gradient_accumulation_steps": args.gradient_accumulation_steps,
            "learning_rate": args.learning_rate,
            "lora_r": 16,
            "lora_alpha": 32,
        },
    }

    ledger_path = out_dir / "provenance_ledger.json"
    ledger_path.write_text(json.dumps(provenance, indent=2), encoding="utf-8")
    print(f"[+] Training complete. Provenance ledger sealed at {ledger_path}")

if __name__ == "__main__":
    main()
