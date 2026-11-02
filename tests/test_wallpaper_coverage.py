"""Occlusion is computed as a clipped union, never by counting overlaps twice."""
from pathlib import Path
import json
import subprocess
import unittest
ROOT=Path(__file__).resolve().parents[1]

class Coverage(unittest.TestCase):
    def test_full_partial_overlapping_and_negative_monitor_coordinates(self):
        code='''import {coveredFraction} from %s;
const area={x:-1920,y:-1080,width:1920,height:1080};
const box=(x,y,width,height)=>({x,y,width,height});
const tests=[
 [[],0], [[area],1], [[area,area],1],
 [[box(-1920,-1080,960,1080)],0.5],
 [[box(-1920,-1080,960,1080),box(-960,-1080,960,1080)],1],
 [[box(0,0,100,100)],0],
 [[box(-2000,-1100,2000,1100)],1],
 [[box(-1920,-1080,960,540),box(-960,-1080,960,540),box(-1920,-540,960,540),box(-960,-540,960,540)],1]
];
for(const [rectangles,expected] of tests)if(Math.abs(coveredFraction(area,rectangles)-expected)>1e-9)throw Error(JSON.stringify({rectangles,expected,actual:coveredFraction(area,rectangles)}));
if(coveredFraction({x:0,y:0,width:0,height:10},[])!==0)throw Error('Empty work area');
'''%json.dumps((ROOT/'gnome/lib/window-coverage.js').as_uri())
        subprocess.run(['node','--input-type=module','-e',code],check=True,capture_output=True)

if __name__=='__main__':unittest.main()
