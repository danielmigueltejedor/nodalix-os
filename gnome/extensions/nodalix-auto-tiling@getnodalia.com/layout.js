// A binary split tree retains placement and ratios across reflows.
export const sides = ['left', 'right', 'top', 'bottom'];
export function ids(tree) {
    return !tree ? [] : tree.id !== undefined ? [tree.id] : [...ids(tree.first), ...ids(tree.second)];
}
export function remove(tree, id) {
    if (!tree || tree.id === id)
        return null;
    if (tree.id !== undefined)
        return tree;
    tree.first = remove(tree.first, id);
    tree.second = remove(tree.second, id);
    return !tree.first ? tree.second : !tree.second ? tree.first : tree;
}
export function insert(tree, id, target, side = 'right') {
    const leaf = {id};
    if (!tree)
        return leaf;
    if (target === null || tree.id === target) {
        const before = side === 'left' || side === 'top';
        return {axis: side === 'left' || side === 'right' ? 'x' : 'y', ratio: 0.5,
            first: before ? leaf : tree, second: before ? tree : leaf};
    }
    if (tree.id !== undefined)
        return tree;
    tree.first = insertAt(tree.first, leaf, target, side);
    tree.second = insertAt(tree.second, leaf, target, side);
    return tree;
}
function insertAt(tree, leaf, target, side) {
    return ids(tree).includes(target) ? insert(tree, leaf.id, target, side) : tree;
}
export function swap(tree, a, b) {
    if (!tree)
        return;
    if (tree.id !== undefined) {
        if (tree.id === a) tree.id = b;
        else if (tree.id === b) tree.id = a;
    } else {
        swap(tree.first, a, b);
        swap(tree.second, a, b);
    }
}
export function minimum(tree, gap, sizes = new Map()) {
    if (!tree)
        return {width: 0, height: 0};
    if (tree.id !== undefined)
        return sizes.get(tree.id) ?? {width: 64, height: 64};
    const a = minimum(tree.first, gap, sizes), b = minimum(tree.second, gap, sizes);
    return tree.axis === 'x'
        ? {width: a.width + gap + b.width, height: Math.max(a.height, b.height)}
        : {width: Math.max(a.width, b.width), height: a.height + gap + b.height};
}
export function inset(area, gap) {
    return {x: area.x + gap, y: area.y + gap,
        width: Math.max(1, area.width - 2 * gap), height: Math.max(1, area.height - 2 * gap)};
}
export function rectangles(tree, area, gap, sizes = new Map()) {
    const leaves = new Map(), nodes = new Map();
    function visit(node, rect) {
        if (!node)
            return;
        nodes.set(node, rect);
        if (node.id !== undefined) {
            leaves.set(node.id, rect);
            return;
        }
        const horizontal = node.axis === 'x';
        const length = horizontal ? 'width' : 'height';
        const available = Math.max(2, rect[length] - gap);
        const minA = minimum(node.first, gap, sizes)[length];
        const minB = minimum(node.second, gap, sizes)[length];
        const low = Math.min(minA, available - 1), high = Math.max(low, available - minB);
        const first = Math.max(1, Math.min(available - 1,
            Math.max(low, Math.min(high, Math.round(available * node.ratio)))));
        node.ratio = first / available;
        const a = {...rect, [length]: first};
        const b = {...rect, [length]: available - first};
        if (horizontal) b.x += first + gap;
        else b.y += first + gap;
        visit(node.first, a);
        visit(node.second, b);
    }
    visit(tree, area);
    return {leaves, nodes};
}
function pathTo(tree, id, path = []) {
    if (!tree)
        return null;
    if (tree.id !== undefined)
        return tree.id === id ? path : null;
    return pathTo(tree.first, id, [...path, {node: tree, first: true}]) ??
        pathTo(tree.second, id, [...path, {node: tree, first: false}]);
}
export function resize(tree, id, edges, frame, area, gap, sizes = new Map()) {
    const path = pathTo(tree, id);
    if (!path)
        return false;
    let changed = false;
    for (const edge of edges) {
        const horizontal = edge === 'left' || edge === 'right';
        const wantedFirst = edge === 'right' || edge === 'bottom';
        const boundary = [...path].reverse().find(entry =>
            entry.node.axis === (horizontal ? 'x' : 'y') && entry.first === wantedFirst);
        if (!boundary)
            continue; // Outer monitor borders stay attached to the work area.
        const {nodes} = rectangles(tree, area, gap, sizes);
        const rect = nodes.get(boundary.node);
        const available = (horizontal ? rect.width : rect.height) - gap;
        const coordinate = horizontal
            ? (wantedFirst ? frame.x + frame.width : frame.x - gap)
            : (wantedFirst ? frame.y + frame.height : frame.y - gap);
        boundary.node.ratio = Math.max(0, Math.min(1,
            (coordinate - (horizontal ? rect.x : rect.y)) / available));
        changed = true;
    }
    rectangles(tree, area, gap, sizes); // Enforce all clients' minimum sizes.
    return changed;
}
export function contains(rect, x, y) {
    return x >= rect.x && y >= rect.y && x < rect.x + rect.width && y < rect.y + rect.height;
}
export function dropTarget(leafRects, area, x, y, draggedId) {
    if (!contains(area, x, y))
        return null;
    const edgeZone = Math.min(32, area.width * 0.04, area.height * 0.04);
    const rootDistances = [x - area.x, area.x + area.width - x, y - area.y, area.y + area.height - y];
    const nearest = rootDistances.indexOf(Math.min(...rootDistances));
    if (rootDistances[nearest] < edgeZone)
        return {target: null, side: sides[nearest], root: true};
    for (const [id, rect] of leafRects) {
        if (id === draggedId || !contains(rect, x, y))
            continue;
        const distances = [(x - rect.x) / rect.width, (rect.x + rect.width - x) / rect.width,
            (y - rect.y) / rect.height, (rect.y + rect.height - y) / rect.height];
        const near = distances.indexOf(Math.min(...distances));
        return {target: id, side: Math.min(...distances) < 0.28 ? sides[near] : 'center', root: false};
    }
    return null;
}
export function move(tree, id, target, side) {
    if (target === id)
        return tree;
    if (side === 'center' && ids(tree).includes(id) && ids(tree).includes(target)) {
        swap(tree, id, target);
        return tree;
    }
    return insert(remove(tree, id), id, target, side === 'center' ? 'right' : side);
}
