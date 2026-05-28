#!/usr/bin/env python3
"""
Murmur — Offline LLM Refinement Script
Loads a GGUF model via llama-cpp-python and refines raw transcription text.
Output is JSON on stdout: {"result": "..."} or {"error": "..."}.
"""

import argparse
import json
import sys
import os

def main():
    parser = argparse.ArgumentParser(description="Offline LLM refinement for Murmur")
    parser.add_argument("--model_path", required=True, help="Path to the GGUF model file")
    parser.add_argument("--system_prompt", required=True, help="System prompt for the LLM")
    parser.add_argument("--user_prompt", required=True, help="User prompt containing the raw text to refine")
    parser.add_argument("--thinking", action="store_true", default=False, help="Enable thinking/reasoning mode (Qwen3)")
    parser.add_argument("--n_ctx", type=int, default=2048, help="Context window size")
    parser.add_argument("--max_tokens", type=int, default=1024, help="Max tokens to generate")
    parser.add_argument("--temperature", type=float, default=0.2, help="Sampling temperature")

    args = parser.parse_args()

    # Validate model path
    if not os.path.exists(args.model_path):
        print(json.dumps({"error": f"Model file not found: {args.model_path}"}))
        sys.exit(0)

    try:
        from llama_cpp import Llama
    except ImportError:
        print(json.dumps({"error": "llama-cpp-python is not installed. Please install it via: pip install llama-cpp-python"}))
        sys.exit(0)

    try:
        # Suppress llama.cpp verbose logging
        os.environ["LLAMA_LOG_LEVEL"] = "0"

        # Load model
        llm = Llama(
            model_path=args.model_path,
            n_ctx=args.n_ctx,
            n_threads=max(1, os.cpu_count() // 2) if os.cpu_count() else 4,
            verbose=False,
        )

        # Build system prompt — disable thinking for Qwen3 if not requested
        system_prompt = args.system_prompt
        if not args.thinking:
            system_prompt = system_prompt + "\n\n/no_think"

        # Run chat completion
        response = llm.create_chat_completion(
            messages=[
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": args.user_prompt},
            ],
            temperature=args.temperature,
            max_tokens=args.max_tokens,
        )

        # Extract result
        result_text = response["choices"][0]["message"]["content"]

        # Strip any <think>...</think> blocks that might leak through
        if "<think>" in result_text:
            import re
            result_text = re.sub(r"<think>.*?</think>", "", result_text, flags=re.DOTALL).strip()

        print(json.dumps({"result": result_text.strip()}))

    except Exception as e:
        print(json.dumps({"error": f"LLM inference failed: {str(e)}"}))
        sys.exit(0)


if __name__ == "__main__":
    main()
