"""Compose HellDeck's native MK.2 keys. Requires Pillow and rsvg-convert.
Use: python tools/build_arm_keys.py [generated console PNG]
The generated plate is flattened to an unlettered 144px bezel once; the pod
paths are always read from the original vector so the insignia stays exact.
"""
from pathlib import Path
import sys, io, subprocess
import xml.etree.ElementTree as ET
from PIL import Image, ImageDraw, ImageFilter, ImageOps

OUT=Path(__file__).resolve().parents[1]/'assets/brand'; S=8; SIZE=72*S
if len(sys.argv)>1:
    base=Image.open(sys.argv[1]).convert('RGB').resize((144,144),Image.Resampling.LANCZOS)
    # Only retain the surrounding machined frame; never print model-generated lettering or a substitute emblem.
    ImageDraw.Draw(base).rounded_rectangle((14,14,129,130),radius=7,fill=(5,9,8));base.save(OUT/'arm-console-plate.png')
plate=Image.open(OUT/'arm-console-plate.png').convert('RGB')
logo=ET.parse(OUT/'helldeck.svg').getroot(); ns='{http://www.w3.org/2000/svg}'
paths=next(g for g in logo.findall(ns+'g') if g.get('fill')=='url(#metal)').findall(ns+'path')
assert len(paths)==3
# Squared 1980s terminal lettering, drawn by hand so ARM and PAUSE read at 72px.
font={'A':['01110','11011','11011','11111','11011','11011','11011'],
'R':['11110','11011','11011','11110','11100','11010','11011'],
'M':['10001','11011','11111','11111','11011','11011','11011'],
'P':['11110','11011','11011','11110','11000','11000','11000'],
'U':['11011','11011','11011','11011','11011','11011','01110'],
'S':['01111','11000','11000','01110','00011','00011','11110'],
'E':['11111','11000','11000','11110','11000','11000','11111']}
def key(on):
    hi=(125,255,174) if on else (255,206,86); mid=(53,148,94) if on else (164,111,40); dim=(21,62,45) if on else (74,54,23)
    img=ImageOps.colorize(ImageOps.autocontrast(plate.convert('L'),cutoff=.2),(4,8,9),(70,79,76)).resize((SIZE,SIZE),Image.Resampling.BICUBIC).convert('RGBA');ink=Image.new('RGBA',img.size);d=ImageDraw.Draw(ink)
    def line(pts,c,w=1):d.line([(round(x*S),round(y*S)) for x,y in pts],fill=c,width=round(w*S))
    def rect(x,y,w,h,c):d.rectangle((round(x*S),round(y*S),round((x+w)*S)-1,round((y+h)*S)-1),fill=c)
    # Glass raster and two columns of targeting graduations.
    for y in range(10,49,3): line([(7,y),(65,y)],(14,23,20),.5)
    for x,side in [(8,1),(64,-1)]:
        for y in range(18,46,5):line([(x,y),(x+side*(3 if y==28 else 1.5),y)],dim,.8)
    for i in range(3):rect(8+4*i,5,2.8,1.4,hi if on or i==0 else dim)
    line([(23,5.7),(53,5.7)],dim,.65);rect(57,5,7,1.4,mid)
    # With input armed, the four brackets close around the pod.
    for x,dx in [(13 if on else 8,1),(59 if on else 64,-1)]:
        for y,dy in [(12 if on else 10,1),(46 if on else 48,-1)]:line([(x,y+dy*5),(x,y),(x+dx*5,y)],hi if on else mid,1.4)
    svg=ET.Element('svg',xmlns='http://www.w3.org/2000/svg',viewBox='12 20 104 89');g=ET.SubElement(svg,'g',fill='#%02x%02x%02x'%hi)
    for path in paths:ET.SubElement(g,'path',path.attrib)
    mark=Image.open(io.BytesIO(subprocess.check_output(['rsvg-convert','-w',str(46*S),'-h',str(39*S)],input=ET.tostring(svg)))).convert('RGBA');ink.alpha_composite(mark,(13*S,9*S))
    rect(6,51,60,.9,mid);label='PAUSE' if on else 'ARM';unit=1.55 if on else 2.;y=54;group=(len(label)*6-1)*unit+9;left=(72-group)/2
    if on:rect(left,y+2,2,7,hi);rect(left+4,y+2,2,7,hi)
    else: d.polygon([(round(x*S),round(v*S)) for x,v in [(left,y+3),(left+6,y+7),(left,y+11)]],fill=hi)
    for idx,ch in enumerate(label):
        for gy,row in enumerate(font[ch]):
            for gx,val in enumerate(row):
                if val=='1':rect(left+9+(6*idx+gx)*unit,y+gy*unit,unit,unit,hi)
    glow=ink.filter(ImageFilter.GaussianBlur(S*.8));glow.putalpha(glow.getchannel('A').point(lambda a:a//3));img.alpha_composite(glow);img.alpha_composite(ink)
    return img.convert('RGB').resize((72,72),Image.Resampling.LANCZOS)
a,b=key(False),key(True);a.save(OUT/'arm-standby.png');b.save(OUT/'arm-live.png');proof=Image.new('RGB',(312,152),(12,17,18));proof.paste(a.resize((144,144),Image.Resampling.NEAREST),(6,4));proof.paste(b.resize((144,144),Image.Resampling.NEAREST),(162,4));proof.save(OUT/'arm-key-preview.png')
