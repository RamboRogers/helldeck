"""Build the embedded atlas from the pinned icon set and HellStrat's Markdown tables.
Usage: python tools/import_assets.py [ICONS_DIR TABLES.md]  (Pillow, rsvg-convert)
Reviewed additions and SKU aliases live in assets/catalog-overlay.json. The
import fails on new unmatched loadout rows instead of silently losing a warbond.
"""
import io
import json
import re
import subprocess
import sys
from pathlib import Path
from PIL import Image


def norm(s):
    return re.sub('[^a-z0-9]', '', s.lower())


out = Path(__file__).resolve().parents[1] / 'assets'
extra = json.loads((out / 'catalog-overlay.json').read_text())
if len(sys.argv) == 1:
    icon_dir, table = out / 'source/icons', out / 'source/stratagems.md'
elif len(sys.argv) == 3:
    icon_dir, table = map(Path, sys.argv[1:])
else:
    sys.exit('Usage: python tools/import_assets.py [ICONS_DIR TABLES.md]')
icons = {norm(p.stem): p for p in icon_dir.rglob('*.svg')}
alias = {'eaglegasairstrike': 'eaglegasstrike', 'spearlauncher': 'spear',
         'deescalator': 'gl52deescalator', 'portablehellbomb': 'hellbombportable',
         'guarddogdogbreath': 'guarddogbreath', 'gasmines': 'gasmine'}
rows, tiles, seen = [], [], set()


def add(name, code, group, art=None):
    key = norm(name)
    icon = icons.get(norm(art) if art else alias.get(key, key))
    if icon is None:
        if group != 'mission':
            raise ValueError(f'No matching art for {name!r}. Review the import; do not silently omit it.')
        print(f'No paired mission art: {name}', file=sys.stderr)
        return
    sid = re.sub('[^a-z0-9]+', '-', name.lower()).strip('-')
    if sid in seen or not re.fullmatch('[UDLR]{3,12}', code):
        raise ValueError(f'invalid or duplicate entry: {sid}: {code}')
    seen.add(sid)
    rows.append(dict(id=sid, name=name, group=group, code=code))
    png = subprocess.check_output(['rsvg-convert','-w','72','-h','72',str(icon)])
    tile = Image.new('RGB', (72, 72), (12, 15, 18)); art = Image.open(io.BytesIO(png)).convert('RGBA'); tile.paste(art,mask=art.getchannel('A')); tiles.append(tile)


section = None
supplied = set()
for line in table.read_text().splitlines():
    if line.startswith('## '):
        section = line[3:].split()[0].lower()
    m = re.fullmatch(r'\| ([^|]+) \| ((?:UP|DOWN|LEFT|RIGHT)(?: → (?:UP|DOWN|LEFT|RIGHT))+) \|', line)
    if not m:
        continue
    name, dirs = m.groups()
    code = ''.join(x[0] for x in dirs.split(' → '))
    if name in extra:
        if code != extra[name]['code']:
            raise ValueError(f'Conflicting sequences for {name}: {code} / {extra[name]["code"]}')
        supplied.add(name)
        add(name, code, 'loadout', extra[name]['icon'])
    else:
        name = 'Hellbomb' if norm(name) == 'hellbomb' else name
        group = 'utility' if name in ('Reinforce', 'Resupply') else 'mission' if section == 'mission' else 'loadout'
        add(name, code, group)
for name, e in extra.items():
    if name not in supplied:
        add(name, e['code'], 'loadout', e['icon'])
assert len(rows)>40
atlas=Image.new('RGB',(72*8,72*((len(tiles)+7)//8)))
for i,tile in enumerate(tiles): atlas.paste(tile,((i%8)*72,(i//8)*72))
atlas.save(out/'atlas.png',optimize=True)
(out/'catalog.json').write_text(json.dumps(rows,indent=2)+'\n');print('catalog:',len(rows))
