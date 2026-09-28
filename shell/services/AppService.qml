pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Hyprland

// Desktop application catalog + launcher.
// Wraps Quickshell.DesktopEntries: exposes a displayable, name-sorted list and a
// launch() that records usage for frecency ranking (Phase 9c).
QtObject {
    id: root

    // All displayable entries, sorted by name. Each item is a DesktopEntry:
    //   .id .name .genericName .comment .icon .categories .keywords .execute()
    readonly property var apps: {
        const out = []
        const all = DesktopEntries.applications?.values ?? []
        for (let i = 0; i < all.length; i++) {
            const e = all[i]
            if (!e || e.noDisplay) continue
            out.push(e)
        }
        out.sort((a, b) => (a.name ?? "").localeCompare(b.name ?? ""))
        return out
    }

    // Resolve a themed icon path for an entry (fallback: generic exec icon).
    // Resolving freedesktop themed icons can be expensive on a cold lookup.
    // Cache the resolved path for the lifetime of the shell. Launcher warm-up
    // then actually makes later opens/searches cheap.
    property var _iconPathCache: ({})

    function iconFor(entry) {
        const name = entry?.icon || "application-x-executable"

        const cached = _iconPathCache[name]
        if (cached !== undefined)
            return cached

        const start = Date.now()
        const path = name.startsWith("/")
            ? "file://" + name
            : Quickshell.iconPath(name, "application-x-executable")

        _iconPathCache[name] = path
        return path
    }

    function launcherIconCacheKey(name) {
        return String(name || "application-x-executable")
            .replace(/[^A-Za-z0-9._-]/g, "_")
    }

    function launcherCachedIcon(name) {
        return "file://" + Quickshell.env("HOME")
            + "/.cache/nodalix/launcher-icons/"
            + launcherIconCacheKey(name)
            + ".png"
    }

    function launcherCachedIconFor(entry) {
        return launcherCachedIcon(entry?.icon || "application-x-executable")
    }

    function launcherCachedSymbolicIcon(name, darkTheme) {
        const variant = darkTheme ? "symbolic-dark" : "symbolic-light"

        return "file://" + Quickshell.env("HOME")
            + "/.cache/nodalix/launcher-icons/"
            + variant + "/"
            + launcherIconCacheKey(name)
            + ".png"
    }

    // Stable key for usage tracking / pin storage.
    function keyOf(entry) {
        return entry?.id ?? entry?.name ?? ""
    }

    // Launch an app and record the usage event (frecency wired in Phase 9c).
    function launch(entry) {
        if (!entry) return
        AppUsageService.record(keyOf(entry))
        entry.execute()
    }

    // Lookup by stable key (for pinned/recommended which store keys, not objects).
    function byKey(key) {
        const a = apps
        for (let i = 0; i < a.length; i++)
            if (keyOf(a[i]) === key) return a[i]
        return null
    }

    // Best-effort map from a Hyprland window class to a DesktopEntry.
    function byClass(cls) {
        if (!cls) return null
        const c = cls.toLowerCase().replace(/\.desktop$/, "")
        const a = apps
        // 1) StartupWMClass exact
        for (let i = 0; i < a.length; i++) {
            const sc = (a[i].startupClass || "").toLowerCase()
            if (sc && sc === c) return a[i]
        }
        // 2) id exact
        for (let i = 0; i < a.length; i++)
            if ((keyOf(a[i]) || "").toLowerCase() === c) return a[i]
        // 3) id tail match (e.g. org.gnome.Nautilus ↔ nautilus) or name exact
        for (let i = 0; i < a.length; i++) {
            const id = (keyOf(a[i]) || "").toLowerCase()
            if (id.endsWith("." + c) || (a[i].name || "").toLowerCase() === c) return a[i]
        }
        const base = c.replace(/\s+r\d{4}[ab].*$/, "")
        if (base === "matlab") {
            for (let i = 0; i < a.length; i++)
                if (/^matlab(?:\s|$)/i.test(a[i].name || "")) return a[i]
        }
        const lookup = DesktopEntries.heuristicLookup(cls)
        return lookup && !lookup.noDisplay ? lookup : null
    }

    // ── Global usage tracking ───────────────────────────────────────────────--
    // Record a usage event whenever any window opens (not only launcher-initiated
    // launches), so the Recommended section reflects real activity.
    property Connections _winTrack: Connections {
        target: Hyprland
        function onRawEvent(event) {
            if (event.name !== "openwindow") return
            // openwindow data: address,workspace,class,title  (title may contain commas)
            const parts = event.parse(4)
            const cls = parts.length >= 3 ? parts[2] : ""
            const app = root.byClass(cls)
            if (app) AppUsageService.record(root.keyOf(app))
        }
    }

    // ── Fuzzy search ──────────────────────────────────────────────────────────
    // Subsequence match with bonuses for word-boundary and early hits.
    // Returns -1 if `q` is not a subsequence of `text`.
    function _score(q, text) {
        if (!text) return -1
        text = text.toLowerCase()
        let ti = 0, score = 0, first = -1
        for (let qi = 0; qi < q.length; qi++) {
            const c = q[qi]
            let found = -1
            for (; ti < text.length; ti++) {
                if (text[ti] === c) { found = ti; break }
            }
            if (found < 0) return -1
            if (first < 0) first = found
            // word-boundary bonus (start, or after a separator)
            if (found === 0 || " -_./".indexOf(text[found - 1]) >= 0) score += 10
            score += 1
            ti = found + 1
        }
        if (text === q) score += 30          // exact match
        else if (text.indexOf(q) === 0) score += 15  // prefix
        return score - first                  // earlier first hit ranks higher
    }

    readonly property var searchIndex: apps.map(e => ({
        entry: e, name: (e.name || "").toLowerCase(),
        generic: (e.genericName || "").toLowerCase(),
        keywords: (e.keywords || []).join(" ").toLowerCase()
    }))

    // Ranked list of DesktopEntry for a query (best first).
    function search(query) {
        const q = (query || "").trim().toLowerCase()
        if (!q) return []
        const scored = []
        const a = searchIndex
        for (let i = 0; i < a.length; i++) {
            const record = a[i]
            const e = record.entry
            let s = _score(q, record.name)
            const g = _score(q, record.generic)
            if (g > s) s = g
            const kw = record.keywords
            const k = _score(q, kw)
            if (k - 5 > s) s = k - 5         // keyword hits worth slightly less
            if (s >= 0) scored.push({ e: e, s: s })
        }
        scored.sort((x, y) => y.s - x.s || (x.e.name || "").localeCompare(y.e.name || ""))
        return scored.map(x => x.e)
    }
}
