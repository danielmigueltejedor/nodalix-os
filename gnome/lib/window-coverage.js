// Exact union coverage of opaque window rectangles, independent of file/GPU.
export function coveredFraction(area, rectangles) {
    if (area.width <= 0 || area.height <= 0) return 0;
    const boxes = rectangles.map(r => ({
        x1: Math.max(area.x, r.x), y1: Math.max(area.y, r.y),
        x2: Math.min(area.x + area.width, r.x + r.width),
        y2: Math.min(area.y + area.height, r.y + r.height),
    })).filter(r => r.x2 > r.x1 && r.y2 > r.y1);
    const xs = [...new Set(boxes.flatMap(r => [r.x1, r.x2]))].sort((a,b) => a-b);
    let covered = 0;
    for (let i=1; i<xs.length; i++) {
        const bands = boxes.filter(r => r.x1 < xs[i] && r.x2 > xs[i-1])
            .map(r => [r.y1, r.y2]).sort((a,b) => a[0]-b[0]);
        let top=0, bottom=0, height=0, started=false;
        for (const [y1,y2] of bands) {
            if (!started) { top=y1; bottom=y2; started=true; }
            else if (y1 > bottom) { height += bottom-top; top=y1; bottom=y2; }
            else bottom=Math.max(bottom,y2);
        }
        height += bottom-top;
        covered += (xs[i]-xs[i-1])*height;
    }
    return Math.min(1, covered/(area.width*area.height));
}
