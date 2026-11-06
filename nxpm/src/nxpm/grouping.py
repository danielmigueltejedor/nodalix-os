"""Group the same application from several sources and choose one."""

from __future__ import annotations

from nxpm.model import Package, normalize_appstream_id


def group_key(package: Package) -> str:
    appstream_id = normalize_appstream_id(package.appstream_id)
    if appstream_id:
        return appstream_id.casefold()
    return package.id


def choose_preferred(candidates: list[Package], preference: list[str]) -> Package:
    """Prefer Flatpak for graphical apps and official packages otherwise."""

    graphical = [item for item in candidates if item.source == "flatpak" and item.kind == "app"]
    if graphical:
        return graphical[0]
    ordinary = [item for item in candidates if not (item.source == "flatpak" and item.kind == "runtime")]
    pool = ordinary or candidates
    for source in preference:
        for item in pool:
            if item.source == source:
                return item
    return pool[0]


def group_packages(packages: list[Package], preference: list[str]) -> list[Package]:
    buckets: dict[str, list[Package]] = {}
    order: list[str] = []
    for package in packages:
        key = group_key(package)
        if key not in buckets:
            buckets[key] = []
            order.append(key)
        buckets[key].append(package)

    grouped: list[Package] = []
    for key in order:
        candidates = buckets[key]
        preferred = choose_preferred(candidates, preference)
        for candidate in candidates:
            candidate.group_id = key
            candidate.preferred = candidate is preferred
            grouped.append(candidate)
    return grouped


def preferred_only(packages: list[Package]) -> list[Package]:
    """Keep one result per application and mention the other sources."""

    groups: dict[str, list[Package]] = {}
    order: list[str] = []
    for package in packages:
        key = package.group_id or group_key(package)
        if key not in groups:
            groups[key] = []
            order.append(key)
        groups[key].append(package)

    chosen: list[Package] = []
    for key in order:
        candidates = groups[key]
        preferred = next((item for item in candidates if item.preferred), candidates[0])
        others = sorted({item.source for item in candidates if item is not preferred})
        if others:
            note = "Also available from " + ", ".join(others)
            preferred.summary = f"{preferred.summary} · {note}".strip(" ·")
        chosen.append(preferred)
    return chosen
