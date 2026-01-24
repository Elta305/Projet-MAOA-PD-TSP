import sys
from pathlib import Path
import cairosvg

root = Path(__file__).resolve().parents[1]
figs = root / 'report' / 'figs'

if not figs.exists():
    print('Directory not found:', figs)
    sys.exit(1)

svgs = list(figs.glob('*.svg'))
if not svgs:
    print('No SVG files found in', figs)
    sys.exit(0)

for s in svgs:
    out = s.with_suffix('.png')
    try:
        print('Converting', s.name, '->', out.name)
        cairosvg.svg2png(url=str(s), write_to=str(out))
    except Exception as e:
        print('Failed for', s.name, e)

print('Done')
