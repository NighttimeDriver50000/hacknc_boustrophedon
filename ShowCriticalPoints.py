#!/usr/bin/python3

import sys, json
from PIL import Image, ImageDraw

im = Image.open(sys.argv[1])
draw = ImageDraw.Draw(im)
for x, y in json.load(sys.stdin):
    r = 4
    draw.ellipse([x-r,y-r,x+r,y+r], fill=(255, 0, 0))
im.save("CriticalPoints.png")
