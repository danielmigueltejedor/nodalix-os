var catalog = [];
function score(query, text) {
    if (!text) return -1;
    var position = 0, total = 0, first = -1;
    for (var i = 0; i < query.length; i++) {
        var found = text.indexOf(query[i], position);
        if (found < 0) return -1;
        if (first < 0) first = found;
        if (found === 0 || " -_./".indexOf(text[found - 1]) >= 0) total += 10;
        total++;
        position = found + 1;
    }
    if (text === query) total += 30;
    else if (text.indexOf(query) === 0) total += 15;
    return total - first;
}
WorkerScript.onMessage = function(message) {
    if (message.kind === "catalog") { catalog = message.records; return; }
    var query = message.query;
    var matches = [];
    if (query) {
        for (var i = 0; i < catalog.length; i++) {
            var entry = catalog[i];
            var rank = Math.max(score(query, entry.name), score(query, entry.generic), score(query, entry.keywords) - 5);
            if (rank >= 0) matches.push({ index: entry.index, rank: rank, name: entry.name });
        }
        matches.sort(function(a, b) { return b.rank - a.rank || a.name.localeCompare(b.name); });
    }
    WorkerScript.sendMessage({ generation: message.generation, indices: matches.slice(0, 40).map(function(e) { return e.index; }) });
};
