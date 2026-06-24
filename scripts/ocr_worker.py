#!/usr/bin/env python3
"""
CapPix OCR Worker — RapidOCR ONNX Runtime (Standalone EXE)

Communication protocol:
  - One-shot mode (default): stdin = base64 image string, stdout = JSON result, then exit
  - Persistent mode (env CAPPIX_OCR_MODE=persistent): 
    stdin = JSON lines {"image":"<base64>"}, stdout = JSON result per line

Output JSON format (matches Rust OcrResult):
  {
    "text": "full recognized text",
    "blocks": [
      {"text": "block text", "confidence": 0.95, "bbox": [[x1,y1],[x2,y2],[x3,y3],[x4,y4]]}
    ],
    "elapsed": 0.123
  }
"""

import sys
import json
import base64
import time
import os


def init_engine():
    """Initialize RapidOCR engine with default models."""
    try:
        from rapidocr_onnxruntime import RapidOCR
        engine = RapidOCR()
        return engine
    except ImportError as e:
        error_result = {
            "text": "",
            "blocks": [],
            "elapsed": 0,
            "error": f"RapidOCR import failed: {e}"
        }
        sys.stdout.write(json.dumps(error_result, ensure_ascii=False) + "\n")
        sys.stdout.flush()
        sys.exit(1)


def run_ocr(engine, image_b64: str) -> dict:
    """Run OCR on a base64-encoded image."""
    start = time.time()

    try:
        import numpy as np
        import cv2

        # Decode base64 to image
        image_data = base64.b64decode(image_b64)
        nparr = np.frombuffer(image_data, np.uint8)
        img = cv2.imdecode(nparr, cv2.IMREAD_COLOR)

        if img is None:
            return {
                "text": "",
                "blocks": [],
                "elapsed": time.time() - start,
                "error": "Failed to decode image"
            }

        # Run OCR
        result, elapse = engine(img)

        elapsed = time.time() - start

        if result is None or len(result) == 0:
            return {
                "text": "",
                "blocks": [],
                "elapsed": round(elapsed, 3)
            }

        # Parse RapidOCR result format:
        # result = [[bbox, text, confidence], ...]
        # bbox = [[x1,y1], [x2,y2], [x3,y3], [x4,y4]]
        blocks = []
        full_text_parts = []

        for item in result:
            bbox, text, confidence = item
            blocks.append({
                "text": text,
                "confidence": round(float(confidence), 4),
                "bbox": [[int(p[0]), int(p[1])] for p in bbox]
            })
            full_text_parts.append(text)

        return {
            "text": "\n".join(full_text_parts),
            "blocks": blocks,
            "elapsed": round(elapsed, 3)
        }

    except Exception as e:
        return {
            "text": "",
            "blocks": [],
            "elapsed": time.time() - start,
            "error": str(e)
        }


def mode_oneshot(engine):
    """One-shot mode: read entire stdin as base64, output JSON, exit."""
    image_b64 = sys.stdin.read().strip()
    if not image_b64:
        result = {"text": "", "blocks": [], "elapsed": 0, "error": "No input data"}
    else:
        result = run_ocr(engine, image_b64)

    sys.stdout.write(json.dumps(result, ensure_ascii=False) + "\n")
    sys.stdout.flush()


def mode_persistent(engine):
    """Persistent mode: read JSON lines from stdin, write JSON results to stdout."""
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            request = json.loads(line)
            image_b64 = request.get("image", "")
        except json.JSONDecodeError:
            result = {"text": "", "blocks": [], "elapsed": 0, "error": "Invalid JSON input"}
            sys.stdout.write(json.dumps(result, ensure_ascii=False) + "\n")
            sys.stdout.flush()
            continue

        result = run_ocr(engine, image_b64)
        sys.stdout.write(json.dumps(result, ensure_ascii=False) + "\n")
        sys.stdout.flush()


def main():
    # Log to stderr so it doesn't interfere with stdout protocol
    print("[ocr_worker] Initializing RapidOCR engine...", file=sys.stderr)
    engine = init_engine()
    print("[ocr_worker] Engine ready.", file=sys.stderr)

    mode = os.environ.get("CAPPIX_OCR_MODE", "oneshot")

    if mode == "persistent":
        print("[ocr_worker] Running in persistent mode.", file=sys.stderr)
        mode_persistent(engine)
    else:
        mode_oneshot(engine)


if __name__ == "__main__":
    main()
