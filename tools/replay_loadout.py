#!/usr/bin/env python3
"""Replay labeled PNG icons through the release CLI without opening the deck or training it.

Input: a private directory (PNG filenames are loadout IDs). Use a fresh output
folder outside the repo. Manifest includes slot, label, row hash and transform;
repeat with --replay to evaluate identical pixels with a different binary.
These are rendered fixtures, not independent HUD captures.
"""
import argparse, csv, hashlib, json, os, random, re, subprocess
from collections import Counter
from pathlib import Path
from PIL import Image, ImageFilter


def generate(src, out, ids):
    files = sorted(src.glob('*.png'))
    if len(files) < 4 or any(f.stem not in ids for f in files):
        raise ValueError('expected at least four PNGs named by catalog loadout ID')
    records = []
    for index, treatment in enumerate(('clean', 'dim-blur-shift', 'small-offset', 'larger-soft')):
        order = files.copy()
        random.Random(1829 + index).shuffle(order)
        for n in range(0, len(order), 4):
            row = Image.new('RGB', (452, 112), (25,31,39))
            labels=[]
            for j in range(4):
                f = order[(n+j) % len(order)]
                w, dx, dy, bright, blur = [(89,12,11,1.,0.), (75,17+2*j,19-j,.71,.65),
                                           (67+2*j,11+j,23-j,.83,.35), (93-j,10+j,5+j,.92,.85)][index]
                im=Image.open(f).convert('RGBA'); bg=Image.new('RGB',im.size); bg.paste(im,mask=im.getchannel('A'))
                tile=bg.resize((w,w),Image.Resampling.BILINEAR).filter(ImageFilter.GaussianBlur(blur)).point(lambda x:int(x*bright))
                row.paste(tile,(113*j+dx,dy)); labels.append(f.stem)
            name=f'{index}-{n}.png';row.save(out/name)
            records.append(dict(row=name,sha256=hashlib.sha256((out/name).read_bytes()).hexdigest(),
                                labels=labels, scored=min(4,len(order)-n), treatment=treatment))
    (out/'manifest.json').write_text(json.dumps(records, indent=2)+'\n')


def replay(directory,binary,profile,report):
    records=json.loads((directory/'manifest.json').read_text()); results=[]
    if (profile/'helldeck/hud-v1.json').exists(): raise ValueError('use an isolated profile without learned HUD samples')
    for row in records:
        f=directory/row['row']
        if hashlib.sha256(f.read_bytes()).hexdigest()!=row['sha256']:raise ValueError(f'fixture changed: {f}')
        q=subprocess.run([binary,'recognize',f],env=dict(os.environ,XDG_CONFIG_HOME=str(profile.resolve())),capture_output=True,text=True)
        slots=re.findall(r'^Sync slot (\d+): ([^;]+); (.*)$',q.stderr,re.M)
        if [int(n) for n,_,_ in slots]!=[1,2,3,4] or q.returncode not in (0,1):raise RuntimeError(q.stderr)
        for j,(_,reason,top) in enumerate(slots[:row['scored']]):
            truth=row['labels'][j]; winner=top.partition('=')[0]; decision='review'
            if reason.startswith('accepted'): decision='correct' if truth==winner else 'wrong'
            results.append([row['treatment'],row['row'],j+1,truth,decision,reason,top])
    with report.open('x',newline='') as f:
        w=csv.writer(f);w.writerow(['treatment','row','slot','label','result','reason','candidates']);w.writerows(results)
    for name in dict.fromkeys(r[0] for r in results): print(name,dict(Counter(r[4] for r in results if r[0]==name)))
    return results

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__); p.add_argument('fixtures',type=Path);p.add_argument('report',type=Path);p.add_argument('--icons',type=Path,help='generate fixtures in a new directory; omit to replay an existing manifest');p.add_argument('--binary',type=Path,default=Path('target/release/helldeck'));p.add_argument('--profile',type=Path,required=True);args=p.parse_args()
    if args.icons:
        if args.fixtures.resolve().is_relative_to(Path(__file__).resolve().parents[1]):p.error('keep private game art outside the checkout')
        args.fixtures.mkdir(mode=0o700,parents=True,exist_ok=False)
        cat=json.loads((Path(__file__).resolve().parents[1]/'assets/catalog.json').read_text());generate(args.icons,args.fixtures,{x['id'] for x in cat if x['group']=='loadout'})
    replay(args.fixtures,args.binary.resolve(),args.profile,args.report)
