#!/usr/bin/env python3
"""Optional offline identity audit. Requires Python, numpy, Pillow, rsvg-convert and
an existing local set of named borderless SVGs. Does not download or write art.
The shipped mapping is explicit; this script tests it, never infers or changes it.
"""
import argparse
import csv
import hashlib
import io
from pathlib import Path
import subprocess

import numpy as np
from PIL import Image

from extract_native_refs import PREFIX, ROOT, check, render, reviewed_input, reviewed_keys

# Pixel-identical aliases that are not competing loadout IDs. Do not ignore
# unidentified ties, particularly DropoffFrv vs DropoffFrvSupply (both omitted).
ALIASES = {'EagleStrafe': {'EagleCAS'}, 'HellbombBackpack': {'MissionHellbombBackpack'}}


def describe(png):
    with Image.open(io.BytesIO(png)) as im:
        rgba = im.convert('RGBA')
    digest = hashlib.sha256(rgba.tobytes()).hexdigest()
    bg = Image.new('RGBA', rgba.size, (43, 43, 43, 255))
    bg.alpha_composite(rgba)
    rgb = np.asarray(bg.convert('RGB').resize((96, 96), Image.Resampling.BILINEAR)).astype(np.int16)
    hi, lo = rgb.max(axis=2), rgb.min(axis=2)
    highlight = (lo > 165) & (hi - lo < 48)
    colored = (hi - lo > 50) & (hi > 106)
    return np.stack((highlight, colored)), digest


def dice(a, b, denominator=None):
    if denominator is None:
        denominator = a.sum(axis=1)[:, None] + b.sum(axis=1)[None, :]
    return 2 * (a @ b.T) / np.maximum(1, denominator)


def flattened(a):
    return a.reshape(len(a), -1)


def matrices(n, w):
    white, color = [dice(flattened(n[:, i]), flattened(w[:, i])) for i in (0, 1)]
    # Mine artwork shares a colored strip of ground. Compare the symbol above
    # the ground: all white and upper-half chromatic pixels (the native tank
    # glyph is green; the wiki draws it white). Permit only vertical translation
    # +/-8px. Keep full areas in the denominator, including out-of-frame pixels.
    def mine(a):
        return np.maximum(a[:, 0], a[:, 1] * (np.arange(96)[:, None] < 48))
    a, b = mine(n), mine(w)
    denominator = a.sum((1, 2))[:, None] + b.sum((1, 2))[None, :]
    shifts = []
    for y in range(-8, 9):
        aa = a[:, max(0, -y):min(96, 96-y)]
        bb = b[:, max(0, y):min(96, 96+y)]
        shifts.append(dice(flattened(aa), flattened(bb), denominator))
    return ({'two_masks': (white+color)/2, 'weapon_outline': color, 'mine_glyph': np.max(shifts, axis=0)},
            white, np.argmax(shifts, axis=0)-8)


def audit(xaml, wiki, metrics):
    check(not metrics.resolve().is_relative_to(ROOT), 'write the private audit outside the checkout')
    templates, _ = reviewed_input(xaml)
    mapping = reviewed_keys()
    refs = sorted(p.stem for p in wiki.glob('*.svg'))
    check(set(refs) == {r['catalog_id'] for r in mapping} | {'gunner-frv', 'supply-frv'},
          'expected all 94 catalog-ID borderless SVGs, including both excluded FRVs, for the negative controls')
    keys = sorted(k.removeprefix(PREFIX) for k in templates)
    native, pixel_hash = zip(*(describe(render(templates[PREFIX+k])) for k in keys))
    wmasks, wsha = [], []
    for name in refs:
        source = (wiki/(name+'.svg')).read_bytes()
        wsha.append(hashlib.sha256(source).hexdigest())
        # Pipe a local trusted SVG to librsvg, with no path/base URI.
        png = subprocess.run(['rsvg-convert', '-w', '256', '-h', '256'], input=source,
                             check=True, capture_output=True).stdout
        wmasks.append(describe(png)[0])
    scores, white, dy = matrices(np.stack(native).astype('float32'), np.stack(wmasks).astype('float32'))
    rows = []
    for row in mapping:
        key, name, method = row['template'], row['catalog_id'], row['verification']
        i, j = keys.index(key), refs.index(name)
        aliases = {k for k, h in zip(keys, pixel_hash) if h == pixel_hash[i] and k != key}
        check(aliases == ALIASES.get(key, set()), f'{key}: unexpected identical art: {aliases}')
        s = scores[method]
        k2 = max((k for k in range(len(keys)) if k != i and keys[k] not in aliases), key=lambda k: s[k, j])
        w2 = max((w for w in range(len(refs)) if w != j), key=lambda w: s[i, w])
        result = bool(s[i, j] >= .80 and s[i, j] > s[i, w2] and s[i, j] > s[k2, j])
        rows.append(dict(catalog_id=name, template=key, method=method, passed=result,
                         score=f'{s[i,j]:.6f}', other_wiki=refs[w2], other_wiki_score=f'{s[i,w2]:.6f}',
                         other_native=keys[k2], other_native_score=f'{s[k2,j]:.6f}',
                         same_pixels_native_alias=';'.join(sorted(aliases)), fixed_white=f'{white[i,j]:.6f}',
                         fixed_color=f"{scores['weapon_outline'][i,j]:.6f}",
                         mine_shift_y=int(dy[i,j]) if method=='mine_glyph' else '',
                         wiki_file=name+'.svg', wiki_sha256=wsha[j]))
    with metrics.open('x', newline='') as f:
        writer = csv.DictWriter(f, rows[0].keys())
        writer.writeheader()
        writer.writerows(rows)
    failed = [r['catalog_id'] for r in rows if not r['passed']]
    check(not failed, 'failed reviewed mappings: '+', '.join(failed))
    return f'{len(rows)} verified against {len(refs)} named borderless files and {len(keys)} native templates'


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('xaml', type=Path)
    parser.add_argument('borderless', type=Path, help='local named wiki SVG directory for auditing only')
    parser.add_argument('metrics', type=Path, help='new CSV outside the checkout')
    args = parser.parse_args()
    try:
        print(audit(args.xaml, args.borderless, args.metrics))
    except (OSError, ValueError, subprocess.SubprocessError) as err:
        parser.exit(1, f'{err}\n')
