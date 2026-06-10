#!/usr/bin/env python3
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from ranking import (  # noqa: E402
    build_web_search_items,
    rank_search_results,
    score_local_item,
)


def accept_all(_item):
    return True


class RankingTests(unittest.TestCase):
    def test_curs_cursor_before_web(self):
        locals_ = [
            {
                "kind": "app",
                "title": "Cursor",
                "search": "cursor editor code",
                "exec_base": "cursor",
                "category": "Dev",
            },
            {
                "kind": "action",
                "title": "Terminal",
                "search": "terminal shell",
                "category": "Sistema",
            },
        ]
        ranked = rank_search_results("curs", locals_, accept_all)
        kinds_titles = [(i["kind"], i["title"]) for i in ranked]
        self.assertEqual(kinds_titles[0], ("app", "Cursor"))
        self.assertTrue(any(k == "web_search" for k, _ in kinds_titles))
        web_index = next(i for i, (k, _) in enumerate(kinds_titles) if k == "web_search")
        self.assertGreater(web_index, 0)

    def test_firefox_before_web(self):
        locals_ = [
            {
                "kind": "app",
                "title": "Firefox",
                "search": "firefox browser web",
                "exec_base": "firefox",
                "category": "Internet",
            },
        ]
        ranked = rank_search_results("fire", locals_, accept_all)
        self.assertEqual(ranked[0]["title"], "Firefox")
        self.assertEqual(ranked[1]["kind"], "web_search")

    def test_wifi_settings_before_web(self):
        locals_ = [
            {
                "kind": "action",
                "title": "Wi-Fi",
                "search": "wifi red wireless nodalix-settings",
                "category": "Red",
            },
        ]
        ranked = rank_search_results("wifi", locals_, accept_all)
        self.assertEqual(ranked[0]["title"], "Wi-Fi")
        self.assertEqual(ranked[1]["kind"], "web_search")

    def test_no_locals_web_first(self):
        locals_ = [
            {
                "kind": "app",
                "title": "Calculator",
                "search": "calc",
                "exec_base": "gnome-calculator",
                "category": "Utilidades",
            },
        ]
        ranked = rank_search_results("asdkjhaskjdh", locals_, accept_all)
        self.assertEqual(ranked[0]["kind"], "web_search")
        self.assertIn("asdkjhaskjdh", ranked[0]["title"])

    def test_web_title_format(self):
        items = build_web_search_items("curs")
        search = next(i for i in items if i["kind"] == "web_search")
        self.assertEqual(search["title"], "Buscar 'curs' en internet")

    def test_score_app_exact_beats_web_tier(self):
        app_score = score_local_item(
            {"kind": "app", "title": "Cursor", "search": "cursor", "exec_base": "cursor"},
            "cursor",
        )
        self.assertGreater(app_score, 100)

    def test_zen_alias_beats_generic_browser_exec(self):
        locals_ = [
            {
                "kind": "app",
                "title": "Navegador",
                "search": "navegador browser web zen",
                "search_aliases": ["browser", "navegador", "web"],
                "exec_base": "zen",
                "category": "Red",
            },
            {
                "kind": "app",
                "title": "Zen Browser",
                "search": "zen browser navegador web zen-browser",
                "search_aliases": ["zen", "zen-browser", "zen browser", "browser", "navegador", "web"],
                "exec_base": "zen-bin",
                "category": "Red",
            },
        ]
        ranked = rank_search_results("zen", locals_, accept_all)
        self.assertEqual(ranked[0]["title"], "Zen Browser")

    def test_browser_aliases_keep_web_after_locals(self):
        locals_ = [
            {
                "kind": "app",
                "title": "Zen Browser",
                "search": "zen browser navegador web",
                "search_aliases": ["zen", "browser", "navegador", "web"],
                "exec_base": "zen-bin",
                "category": "Red",
            },
        ]
        ranked = rank_search_results("navegador", locals_, accept_all)
        self.assertEqual(ranked[0]["title"], "Zen Browser")
        self.assertEqual(ranked[1]["kind"], "web_search")

    def test_natural_question_gets_assistant_before_web(self):
        ranked = rank_search_results("dónde vive jose", [], accept_all)
        self.assertEqual(ranked[0]["title"], "Preguntar a Nodalix Assistant")
        self.assertEqual(ranked[1]["kind"], "web_search")


if __name__ == "__main__":
    unittest.main()
