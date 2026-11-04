import ShakeGesture from '../gnome/extensions/nodalix-shake-pointer@getnodalia.com/gesture.js';

function detects(samples, interval) {
    const gesture = new ShakeGesture();
    return samples.some(([x, y], index) => gesture.push(x, y, index * interval));
}

const horizontal = [[0, 0], [50, 0], [0, 0], [50, 0], [0, 0]];
const vertical = horizontal.map(([x, y]) => [y, x]);
const straight = horizontal.map((_, index) => [50 * index, 0]);
const jitter = horizontal.map(([x, y]) => [x / 10, y]);
if (!detects(horizontal, 40) || !detects(vertical, 40) || detects(straight, 40) ||
    detects(jitter, 40) || detects(horizontal, 180))
    throw new Error('Unexpected pointer shake classification');
