"""Search ranking for Nodalix Command Bar — locals first, web as fallback."""

from __future__ import annotations

from urllib.parse import quote

MatchQuality = str  # exact | starts | partial | fuzzy

SETTINGS_CATEGORIES = frozenset(
    {
        "Nodalix",
        "Red",
        "Wi-Fi",
        "Audio",
        "Apariencia",
        "Sistema",
        "Energía",
        "Bluetooth",
    }
)

SETTINGS_SEARCH_HINTS = (
    "settings",
    "ajustes",
    "wifi",
    "wi-fi",
    "bluetooth",
    "red",
    "apariencia",
    "nodalix-settings",
)


def normalize_web_url(query: str) -> str | None:
    text = query.strip()
    if not text or " " in text:
        return None
    if text.startswith(("http://", "https://")):
        return text
    if "." in text and not text.startswith("."):
        return "https://" + text
    return None


def web_search_url(query: str) -> str:
    return "https://www.google.com/search?q=" + quote(query.strip())


def build_web_search_items(query: str) -> list[dict]:
    text = query.strip()
    if len(text) < 2:
        return []

    items: list[dict] = []
    direct = normalize_web_url(text)
    if direct:
        items.append(
            {
                "kind": "web_open",
                "icon": "󰖟",
                "title": f"Abrir {direct}",
                "subtitle": "Internet · navegador predeterminado",
                "search": text.lower(),
                "url": direct,
                "category": "Web",
            }
        )

    items.append(
        {
            "kind": "web_search",
            "icon": "󰍉",
            "title": f"Buscar '{text}' en internet",
            "subtitle": "Internet · navegador predeterminado",
            "search": text.lower(),
            "url": web_search_url(text),
            "category": "Web",
        }
    )
    return items


def looks_like_assistant_query(query: str) -> bool:
    if len(query.strip()) < 4:
        return False
    question_words = (
        "donde",
        "dónde",
        "quien",
        "quién",
        "que ",
        "qué ",
        "cuando",
        "cuándo",
        "busca ",
        "resume ",
        "recuerdame",
        "recuérdame",
        "añade ",
        "agrega ",
    )
    text = query.strip().lower()
    return "?" in text or any(text.startswith(word) for word in question_words)


def build_assistant_item(query: str) -> dict | None:
    text = query.strip()
    if not looks_like_assistant_query(text):
        return None
    return {
        "kind": "action",
        "icon": "󰚩",
        "title": f"Preguntar a Nodalix Assistant",
        "subtitle": "Intelligence · local-first, permisos explícitos",
        "search": f"assistant asistente ia intelligence {text}".lower(),
        "command": "alacritty -e nodalix-assistant ask " + quote_shell(text),
        "category": "IA",
    }


def quote_shell(value: str) -> str:
    return "'" + value.replace("'", "'\"'\"'") + "'"


def fuzzy_subsequence(needle: str, haystack: str) -> bool:
    if not needle:
        return False
    idx = 0
    for char in haystack:
        if char == needle[idx]:
            idx += 1
            if idx == len(needle):
                return True
    return False


def match_quality(item: dict, query: str) -> MatchQuality | None:
    if not query:
        return None

    title = str(item.get("title", "")).lower()
    exec_base = str(item.get("exec_base", "")).lower()
    search = str(item.get("search", "")).lower()
    aliases = [
        str(alias).lower().strip()
        for alias in item.get("search_aliases", [])
        if str(alias).strip()
    ]

    if title == query or query in aliases:
        return "exact"
    if (
        title.startswith(query)
        or exec_base == query
        or exec_base.startswith(query)
        or any(alias.startswith(query) for alias in aliases)
    ):
        return "starts"
    if query in title or query in exec_base or query in search:
        return "partial"
    if fuzzy_subsequence(query, title) or fuzzy_subsequence(query, search):
        return "fuzzy"
    return None


def is_settings_item(item: dict) -> bool:
    if item.get("kind") != "action":
        return False
    category = str(item.get("category", ""))
    if category in SETTINGS_CATEGORIES:
        return True
    search = str(item.get("search", "")).lower()
    title = str(item.get("title", "")).lower()
    return any(
        hint in search or hint in title for hint in SETTINGS_SEARCH_HINTS
    )


def _tier_score(kind: str, quality: MatchQuality, settings: bool) -> int:
    if kind == "app":
        return {
            "exact": 1000,
            "starts": 900,
            "partial": 700,
            "fuzzy": 500,
        }[quality]

    if kind == "action":
        if settings:
            return {
                "exact": 800,
                "starts": 800,
                "partial": 650,
                "fuzzy": 450,
            }[quality]
        return {
            "exact": 850,
            "starts": 850,
            "partial": 650,
            "fuzzy": 450,
        }[quality]

    # windows / workspaces
    return {
        "exact": 600,
        "starts": 550,
        "partial": 500,
        "fuzzy": 400,
    }[quality]


def _idle_priority(item: dict) -> int:
    return {
        "app": 30,
        "action": 20,
        "window": 10,
        "workspace": 5,
    }.get(item.get("kind"), 0)


def score_local_item(item: dict, query: str) -> int:
    kind = item.get("kind")
    if kind in ("web_search", "web_open"):
        return -1

    if not query:
        return _idle_priority(item)

    quality = match_quality(item, query)
    if quality is None:
        return -1

    score = _tier_score(kind, quality, is_settings_item(item))

    # Prefer available actions over disabled ones at the same tier.
    if item.get("disabled"):
        score -= 50

    return score


def rank_search_results(
    query: str,
    items: list[dict],
    matches_filter,
    *,
    max_results: int = 40,
) -> list[dict]:
    """Rank local providers first; web search is secondary when locals match."""
    normalized = query.strip().lower()

    local_scored: list[tuple[int, int, str, dict]] = []
    for item in items:
        if not matches_filter(item):
            continue
        score = score_local_item(item, normalized)
        if normalized and score < 0:
            continue
        title_key = str(item.get("title", "")).lower()
        priority = int(item.get("search_priority", 0) or 0)
        local_scored.append((score, priority, title_key, item))

    local_scored.sort(key=lambda row: (-row[0], -row[1], row[2]))
    local_results = [item for _score, _priority, _title, item in local_scored]

    if not normalized or len(normalized) < 2:
        return local_results[:max_results]

    assistant_item = build_assistant_item(query)
    if assistant_item is not None and matches_filter(assistant_item):
        local_scored.append((300, 0, assistant_item["title"].lower(), assistant_item))
        local_scored.sort(key=lambda row: (-row[0], -row[1], row[2]))
        local_results = [item for _score, _priority, _title, item in local_scored]

    web_items = build_web_search_items(normalized)
    has_relevant_locals = len(local_results) > 0
    web_base = 1000 if not has_relevant_locals else 100

    web_scored: list[tuple[int, int, str, dict]] = []
    for index, web_item in enumerate(web_items):
        # Direct URL slightly above generic web search when both exist.
        bonus = 1 if web_item.get("kind") == "web_open" else 0
        web_scored.append((web_base + bonus - index, 0, web_item.get("title", ""), web_item))

    combined = local_scored + web_scored
    combined.sort(key=lambda row: (-row[0], -row[1], row[2]))
    return [item for _score, _priority, _title, item in combined[:max_results]]
