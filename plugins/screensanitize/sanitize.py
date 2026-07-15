import argparse, base64, json, os, re, subprocess
from PIL import Image

p = argparse.ArgumentParser()
p.add_argument("--input", required=True)
p.add_argument("--output", required=True)
a = p.parse_args()
results = []
for root, _, files in os.walk(a.input):
    for name in files:
        if not name.lower().endswith((".png", ".jpg", ".jpeg", ".webp")):
            continue
        path = os.path.join(root, name)
        item = {"file": os.path.relpath(path, a.input), "untrusted": True, "flags": []}
        try:
            with Image.open(path) as image:
                clean = image.convert("RGB")
                clean.save(path + ".sanitized.jpg", format="JPEG", quality=85, optimize=True)
            text = subprocess.check_output(["tesseract", path, "stdout"], stderr=subprocess.DEVNULL, timeout=30, text=True)
            item["ocr_text"] = text[:8192]
            if re.search(r"[\u200b-\u200f\u202a-\u202e\ufeff]", text): item["flags"].append("unicode_controls")
            if re.search(r"(?:[A-Za-z0-9+/]{24,}={0,2})", text): item["flags"].append("base64_like")
            if re.search(r"(?:[.-]{3,}\s+){2,}", text): item["flags"].append("morse_like")
            if any("\u2800" <= c <= "\u28ff" for c in text): item["flags"].append("braille")
            if re.search(r"ignore (?:all|previous)|system message|developer message|tool call", text, re.I): item["flags"].append("prompt_injection_like")
        except Exception as e:
            item["error"] = str(e)
        results.append(item)
with open(a.output, "w", encoding="utf-8") as f:
    json.dump({"untrusted": True, "results": results}, f, indent=2)
