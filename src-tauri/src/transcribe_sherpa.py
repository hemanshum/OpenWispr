#!/usr/bin/env python3
import sys
import os
import argparse
import json
import wave
import numpy as np
import ctypes

# Force loading bundled onnxruntime.dll to avoid conflicts with System32 dll
def _load_bundled_onnxruntime():
    try:
        import importlib.util
        spec = importlib.util.find_spec("sherpa_onnx")
        paths = []
        if spec and spec.submodule_search_locations:
            paths.extend(spec.submodule_search_locations)
        for p in sys.path:
            paths.append(os.path.join(p, "sherpa_onnx"))
        for base_path in paths:
            dll_path = os.path.join(base_path, "lib", "onnxruntime.dll")
            if os.path.exists(dll_path):
                if sys.platform == "win32" and hasattr(os, "add_dll_directory"):
                    os.add_dll_directory(os.path.dirname(dll_path))
                try:
                    ctypes.CDLL(dll_path)
                    return True
                except Exception:
                    pass
    except Exception:
        pass
    return False

_load_bundled_onnxruntime()

try:
    import sherpa_onnx
except ImportError:
    print(json.dumps({"error": "sherpa-onnx is not installed."}))
    sys.exit(1)

def main():
    parser = argparse.ArgumentParser(description="Offline transcription via sherpa-onnx")
    parser.add_argument("--wav_path", required=True, help="Path to input .wav file (16kHz, 16bit, mono)")
    parser.add_argument("--model_type", choices=["whisper", "parakeet"], required=True, help="Model family")
    parser.add_argument("--model_dir", required=True, help="Directory containing model files")
    parser.add_argument("--language", default="auto", help="Language code or 'auto'")
    args = parser.parse_args()

    if not os.path.exists(args.wav_path):
        print(json.dumps({"error": f"WAV file not found: {args.wav_path}"}))
        return

    tokens_path = None
    encoder_path = None
    decoder_path = None
    joiner_path = None

    if args.model_type == "whisper":
        # Find whisper files
        for f in os.listdir(args.model_dir):
            if f.endswith("encoder.int8.onnx"):
                encoder_path = os.path.join(args.model_dir, f)
            elif f.endswith("decoder.int8.onnx"):
                decoder_path = os.path.join(args.model_dir, f)
            elif f.endswith("tokens.txt"):
                tokens_path = os.path.join(args.model_dir, f)
    elif args.model_type == "parakeet":
        encoder_path = os.path.join(args.model_dir, "encoder.int8.onnx")
        decoder_path = os.path.join(args.model_dir, "decoder.int8.onnx")
        joiner_path = os.path.join(args.model_dir, "joiner.int8.onnx")
        tokens_path = os.path.join(args.model_dir, "tokens.txt")

    if not encoder_path or not os.path.exists(encoder_path):
        print(json.dumps({"error": f"Encoder model not found in {args.model_dir}"}))
        return
    if not decoder_path or not os.path.exists(decoder_path):
        print(json.dumps({"error": f"Decoder model not found in {args.model_dir}"}))
        return
    if not tokens_path or not os.path.exists(tokens_path):
        print(json.dumps({"error": f"Tokens file not found in {args.model_dir}"}))
        return

    try:
        recognizer = None
        # Try direct class methods first (recommended/modern API)
        try:
            if args.model_type == "whisper":
                lang = args.language if args.language != "auto" else ""
                recognizer = sherpa_onnx.OfflineRecognizer.from_whisper(
                    encoder=encoder_path,
                    decoder=decoder_path,
                    tokens=tokens_path,
                    language=lang,
                    task="transcribe",
                    num_threads=4,
                )
            elif args.model_type == "parakeet":
                if not joiner_path or not os.path.exists(joiner_path):
                    print(json.dumps({"error": f"Joiner model not found for Parakeet in {args.model_dir}"}))
                    return
                recognizer = sherpa_onnx.OfflineRecognizer.from_transducer(
                    encoder=encoder_path,
                    decoder=decoder_path,
                    joiner=joiner_path,
                    tokens=tokens_path,
                    model_type="nemo_transducer",
                    num_threads=4,
                )
        except Exception as e_direct:
            # Fallback to the OfflineRecognizerConfig approach if class methods fail
            try:
                whisper_config = None
                transducer_config = None
                
                if args.model_type == "whisper":
                    lang = args.language if args.language != "auto" else ""
                    whisper_config = sherpa_onnx.OfflineWhisperModelConfig(
                        encoder=encoder_path,
                        decoder=decoder_path,
                        language=lang,
                        task="transcribe",
                    )
                    model_config = sherpa_onnx.OfflineModelConfig(
                        whisper=whisper_config,
                        tokens=tokens_path,
                        num_threads=4,
                        debug=False,
                    )
                else:
                    if not joiner_path or not os.path.exists(joiner_path):
                        print(json.dumps({"error": f"Joiner model not found for Parakeet in {args.model_dir}"}))
                        return
                    transducer_config = sherpa_onnx.OfflineTransducerModelConfig(
                        encoder_filename=encoder_path,
                        decoder_filename=decoder_path,
                        joiner_filename=joiner_path,
                    )
                    model_config = sherpa_onnx.OfflineModelConfig(
                        transducer=transducer_config,
                        tokens=tokens_path,
                        model_type="nemo_transducer",
                        num_threads=4,
                        debug=False,
                    )

                feat_config = sherpa_onnx.FeatureExtractorConfig(
                    sampling_rate=16000,
                    feature_dim=80,
                )
                
                try:
                    recognizer_config = sherpa_onnx.OfflineRecognizerConfig(
                        feat_config=feat_config,
                        model_config=model_config,
                    )
                    recognizer = sherpa_onnx.OfflineRecognizer(recognizer_config)
                except Exception:
                    try:
                        recognizer_config = sherpa_onnx.OfflineRecognizerConfig(
                            feat_config=feat_config,
                            model=model_config,
                        )
                        recognizer = sherpa_onnx.OfflineRecognizer(recognizer_config)
                    except Exception:
                        recognizer = sherpa_onnx.OfflineRecognizer(
                            tokens=tokens_path,
                            whisper=whisper_config if args.model_type == "whisper" else None,
                            transducer=transducer_config if args.model_type == "parakeet" else None,
                            model_type="nemo_transducer" if args.model_type == "parakeet" else "",
                            num_threads=4,
                        )
            except Exception as e_fallback:
                print(json.dumps({"error": f"Failed to initialize OfflineRecognizer: {e_fallback} (Direct error: {e_direct})"}))
                return

        # Load WAV file
        with wave.open(args.wav_path, "rb") as f:
            num_channels = f.getnchannels()
            sample_width = f.getsampwidth()
            framerate = f.getframerate()
            num_frames = f.getnframes()
            
            raw_data = f.readframes(num_frames)

        # Convert to float32
        if sample_width == 2:
            data = np.frombuffer(raw_data, dtype=np.int16).astype(np.float32) / 32768.0
        elif sample_width == 1:
            data = (np.frombuffer(raw_data, dtype=np.uint8).astype(np.float32) - 128.0) / 128.0
        elif sample_width == 4:
            data = np.frombuffer(raw_data, dtype=np.int32).astype(np.float32) / 2147483648.0
        else:
            print(json.dumps({"error": f"Unsupported sample width: {sample_width}"}))
            return

        if num_channels > 1:
            data = data.reshape(-1, num_channels).mean(axis=1)

        stream = recognizer.create_stream()
        stream.accept_waveform(16000, data)
        recognizer.decode_stream(stream)
        
        result_text = stream.result.text
        print(json.dumps({"text": result_text}))

    except Exception as e:
        print(json.dumps({"error": f"Error running recognition: {str(e)}"}))
        return

if __name__ == "__main__":
    main()
