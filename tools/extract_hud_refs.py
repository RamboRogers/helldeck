#!/usr/bin/env python3
"""Build a private matcher bank from Filediver PNG exports of build 26568081's HUD textures.

Only the identity, resource names, palette and digests ship. No network or game access.
"""
import argparse
import csv
import hashlib
import json
import tempfile
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]


def read_mask(folder, key):
    path = folder / key['export_png']
    if not path.is_file():
        path = folder / (key['texture_hash'] + '.png')
    with Image.open(path, formats=['PNG']) as im:
        if im.size != (256, 256):
            raise ValueError(f'{path}: expected a 256×256 HUD texture')
        rgb = im.convert('RGB')
    if hashlib.sha256(rgb.tobytes()).hexdigest() != key['rgb_sha256']:
        raise ValueError(f"{key['catalog_id']}: unreviewed pixels in {path}")
    # These are packed mask weights, not framebuffer RGB. Apply the category
    # color to red and light detail to green using the verified texture geometry.
    weights = np.asarray(rgb, dtype=np.int32)
    ink = np.array(list(bytes.fromhex(key['ink'])), dtype=np.int32)
    values = 43 + ((ink - 43) * weights[:, :, 0:1] + (237 - 43) * weights[:, :, 1:2] + 127) // 255
    return Image.fromarray(np.uint8(np.clip(values, 0, 255)))


def extract(folder, output):
    with (ROOT / 'tools/hud_texture_keys.csv').open(newline='') as f:
        keys = list(csv.DictReader(f))
    ids = {r['id'] for r in json.loads((ROOT / 'assets/catalog.json').read_text()) if r['group'] == 'loadout'}
    if ({k['catalog_id'] for k in keys} != ids or len(keys) != len(ids)
            or len({k['rgb_sha256'] for k in keys}) != len(ids)):
        raise ValueError('HUD texture manifest must uniquely cover the loadout catalog')
    output = output.resolve()
    if output.is_relative_to(ROOT) or output.exists():
        raise ValueError('choose a new, private output directory outside the checkout')
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=output.parent) as tmp:
        pack = Path(tmp) / 'pack'
        pack.mkdir(mode=0o700)
        icons = pack / 'icons'
        icons.mkdir()
        for key in keys:
            read_mask(folder, key).save(icons / (key['catalog_id'] + '.png'))
        pack.rename(output)
    return len(keys)


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('exports', type=Path, help='Filediver texture export root, named paths or 0xHASH.png')
    p.add_argument('output', type=Path, help='new private output directory')
    args = p.parse_args()
    try:
        print(f'{extract(args.exports, args.output)} verified HUD texture references in {args.output}/icons')
    except (OSError, ValueError) as error:
        p.exit(1, f'{error}\n')
