#!/usr/bin/env python3
"""Render reviewed stratagem templates from a local, already-extracted XAML dictionary.

Private/offline; requires Python 3 and rsvg-convert (librsvg). Not a general XAML
renderer. Never evaluates XAML, resolves resources, downloads files or packs art.
"""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tempfile
import xml.etree.ElementTree as ET

PRESENTATION = '{http://schemas.microsoft.com/winfx/2006/xaml/presentation}'
KEY = '{http://schemas.microsoft.com/winfx/2006/xaml}Key'
PREFIX = 'DataTemplate.Icon.StratagemType.'
NUM = r'[-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][-+]?\d+)?'
ROOT = Path(__file__).resolve().parent.parent
DICTIONARY_SHA256 = '99083ac11f1ae581a26de12d22f0a7f94ea76c22630d5a6992fcfbfae137c315'


def check(condition, reason):
    if not condition:
        raise ValueError(reason)


def color(value):
    if re.fullmatch(r'#[Ff]{2}[0-9a-fA-F]{6}', value):  # XAML #AARRGGBB
        return '#' + value[3:]
    check(value in {'none', 'white', 'black'} or re.fullmatch(r'#[0-9a-fA-F]{3}', value),
          f'unsupported brush: {value}')
    return value


def stretch_bounds(d):
    # Every Stretch=Fill in the reviewed dictionary is a rectangle or an
    # explicitly closed polygon. Reject curves instead of guessing their bounds.
    m = re.fullmatch(rf'M({NUM}),({NUM})h({NUM})v({NUM})h({NUM})z', d)
    if m:
        x, y, w, h, back = map(float, m.groups())
        check(w > 0 and h > 0 and back == -w, 'invalid stretch rectangle')
        return x, y, w, h
    m = re.fullmatch(rf'M{NUM},{NUM}(?:\s*L{NUM},{NUM})+\s*[zZ]', d)
    check(m, 'unsupported stretched path geometry')
    xs, ys = zip(*[(float(x), float(y)) for x, y in re.findall(rf'[ML]({NUM}),({NUM})', d)])
    return min(xs), min(ys), max(xs)-min(xs), max(ys)-min(ys)


def only_child(parent, name):
    check(len(parent) == 1 and parent[0].tag == PRESENTATION + name, f'expected one {name}')
    return parent[0]


def render(template, size=256):
    view = only_child(template, 'Viewbox')
    canvas = only_child(view, 'Canvas')
    check(not view.attrib and set(canvas.attrib) == {'Width', 'Height'}, 'unexpected canvas/viewbox')
    svg = ET.Element('svg', xmlns='http://www.w3.org/2000/svg',
                     viewBox=f"0 0 {float(canvas.attrib['Width'])} {float(canvas.attrib['Height'])}")
    for path in canvas:
        a = path.attrib
        check(path.tag == PRESENTATION+'Path' and set(a) <= {
            'Canvas.Left', 'Canvas.Top', 'Data', 'Fill', 'Stroke', 'StrokeThickness',
            'Stretch', 'Width', 'Height'}, 'unsupported path attribute or element')
        d = re.sub(r'^F[01]\s*', '', a['Data'])
        attrs = {'d': d, 'fill': color(a.get('Fill', 'none')),
                 'fill-rule': 'nonzero' if a['Data'].startswith('F1') else 'evenodd'}
        if 'Stroke' in a:
            attrs.update({'stroke': color(a['Stroke']), 'stroke-width': str(float(a['StrokeThickness'])),
                          'stroke-miterlimit': '10'})
        x, y = (float(a[k]) for k in ('Canvas.Left', 'Canvas.Top'))
        transforms = [f'translate({x} {y})'] if x or y else []
        if len(path):
            transform = only_child(path, 'Path.RenderTransform')
            matrix = only_child(transform, 'MatrixTransform')
            check(not transform.attrib and set(matrix.attrib) == {'Matrix'} and not len(matrix), 'invalid matrix')
            numbers = [float(n) for n in matrix.attrib['Matrix'].split(',')]
            check(len(numbers) == 6, 'invalid matrix size')
            transforms.append('matrix('+' '.join(map(str, numbers))+')')
        if {'Stretch', 'Width', 'Height'} & set(a):
            check(a.get('Stretch') == 'Fill' and 'Stroke' not in a, 'unsupported stretch')
            bx, by, bw, bh = stretch_bounds(d)
            sx, sy = float(a['Width'])/bw, float(a['Height'])/bh
            if sx != 1 or sy != 1:
                transforms.append(f'scale({sx} {sy})')
            if bx or by:
                transforms.append(f'translate({-bx} {-by})')
        if transforms:
            attrs['transform'] = ' '.join(transforms)
        ET.SubElement(svg, 'path', attrs)
    return subprocess.run(['rsvg-convert', '-w', str(size), '-h', str(size)],
                          input=ET.tostring(svg), capture_output=True, check=True).stdout


def reviewed_input(xaml):
    source = xaml.read_bytes()
    check(hashlib.sha256(source).hexdigest() == DICTIONARY_SHA256,
          'unreviewed dictionary; re-audit the mappings before using another build')
    tree = ET.fromstring(source)
    templates = {t.attrib[KEY]: t for t in tree if t.attrib[KEY].startswith(PREFIX)}
    bindings = {}
    for trigger in tree.iter(PRESENTATION+'DataTrigger'):
        for setter in trigger:
            if setter.get('Property') == 'ContentTemplate':
                resource = setter.attrib['Value'].removeprefix('{DynamicResource ').removesuffix('}')
                check(resource not in bindings, 'duplicate trigger binding')
                bindings[resource] = trigger.attrib['Value']
    return templates, bindings


def reviewed_keys():
    with (ROOT/'tools/native_ref_keys.csv').open(newline='') as f:
        rows = list(csv.DictReader(f))
    loadout = {c['id'] for c in json.loads((ROOT/'assets/catalog.json').read_text()) if c['group'] == 'loadout'}
    ids = {r['catalog_id'] for r in rows}
    check(len(rows) == len(ids) == len({r['template'] for r in rows}) and ids <= loadout,
          'duplicate or non-loadout key in reviewed mapping')
    check(all(re.fullmatch(r'[a-z0-9]+(?:-[a-z0-9]+)*', i) for i in ids), 'invalid catalog ID')
    return rows


def extract(xaml, output):
    templates, bindings = reviewed_input(xaml)
    output = output.resolve()
    check(not output.is_relative_to(ROOT), 'private reference output must be outside the checkout')
    check(not output.exists(), 'output directory already exists; use a new private directory')
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=output.parent, prefix='.native-refs-') as scratch:
        stage = Path(scratch)/'pack'
        stage.mkdir(mode=0o700)
        (stage/'icons').mkdir()
        rows = []
        for row in reviewed_keys():
            key = PREFIX+row['template']
            template = templates[key]
            canvas = only_child(only_child(template, 'Viewbox'), 'Canvas')
            data = render(template)
            (stage/'icons'/(row['catalog_id']+'.png')).write_bytes(data)
            rows.append(dict(catalog_id=row['catalog_id'], xaml_key=key, binding=bindings[key],
                             build=26568081, dictionary_sha256=DICTIONARY_SHA256,
                             canvas_width=canvas.attrib['Width'], canvas_height=canvas.attrib['Height'],
                             paths=len(canvas), png_sha256=hashlib.sha256(data).hexdigest(),
                             verification=row['verification']))
        with (stage/'mapping.csv').open('w', newline='') as f:
            writer = csv.DictWriter(f, rows[0].keys())
            writer.writeheader()
            writer.writerows(rows)
        stage.rename(output)
    return len(rows)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('xaml', type=Path, help='local stratagem_icons.xaml (reviewed build 26568081)')
    parser.add_argument('output', type=Path, help='new private directory outside the checkout')
    args = parser.parse_args()
    try:
        print(f'{extract(args.xaml, args.output)} native PNGs in {args.output}/icons; provenance in mapping.csv')
    except (OSError, ValueError, subprocess.SubprocessError) as err:
        parser.exit(1, f'{err}\n')
