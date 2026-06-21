#!/usr/bin/env python3
"""CapPix OCR worker — reads base64 image from stdin, outputs JSON to stdout."""
import sys
import json
import base64
import io

from rapidocr_onnxruntime import RapidOCR

engine = RapidOCR()

def process_image(image_b64: str) -> dict:
    try:
        image_data = base64.b64decode(image_b64)
        import numpy as np
        import cv2
        nparr = np.frombuffer(image_data, np.uint8)
        img = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
        if img is None:
            return {"error": "Failed to decode image"}

        result, elapse = engine(img)
        if result is None:
            return {"text": "", "blocks": [], "elapsed": elapse}

        blocks = []
        full_text_parts = []
        for item in result:
            # item: [bbox, text, confidence]
            bbox, text, confidence = item
            blocks.append({
                "text": text,
                "confidence": round(float(confidence), 3),
                "bbox": [[int(p[0]), int(p[1])] for p in bbox],
            })
            full_text_parts.append(text)

        return {
            "text": "\n".join(full_text_parts),
            "blocks": blocks,
            "elapsed": elapse,
        }
    except Exception as e:
        return {"error": str(e)}

def main():
    # Read base64 from stdin (single line)
    image_b64 = sys.stdin.read().strip()
    if not image_b64:
        print(json.dumps({"error": "No input"}))
        sys.exit(1)

    result = process_image(image_b64)
    print(json.dumps(result, ensure_ascii=False))

if __name__ == "__main__":
    main()
