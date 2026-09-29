#!/usr/bin/env python3

import argparse
from collections import defaultdict
from pathlib import Path

import gi

gi.require_version("ECal", "2.0")
gi.require_version("EDataServer", "1.2")

from gi.repository import ECal, EDataServer

SOURCE_ROOT = Path.home() / ".local/share/vdirsyncer"
EDS_UID = "nodalix-calendar"
EDS_NAME = "Nodalix"


def extract_vevents(text):
    events = []
    pos = 0

    while True:
        begin = text.find("BEGIN:VEVENT", pos)
        if begin < 0:
            break

        end = text.find("END:VEVENT", begin)
        if end < 0:
            break

        end += len("END:VEVENT")
        events.append(text[begin:end] + "\r\n")
        pos = end

    return events


def add_source_file(vevent, relative_path):
    marker = "X-NODALIX-SOURCE-FILE:" + str(relative_path)

    if "\r\nEND:VEVENT" in vevent:
        return vevent.replace(
            "\r\nEND:VEVENT",
            "\r\n" + marker + "\r\nEND:VEVENT",
            1,
        )

    return vevent.replace(
        "\nEND:VEVENT",
        "\n" + marker + "\nEND:VEVENT",
        1,
    )


def connect():
    registry = EDataServer.SourceRegistry.new_sync(None)
    source = registry.ref_source(EDS_UID)

    if source is None:
        source = EDataServer.Source.new_with_uid(
            EDS_UID,
            None,
        )
        source.set_display_name(EDS_NAME)
        source.set_enabled(True)

        calendar = source.get_extension(
            EDataServer.SOURCE_EXTENSION_CALENDAR
        )
        calendar.set_backend_name("local")

        registry.commit_source_sync(
            source,
            None,
        )

        source = registry.ref_source(EDS_UID)

    if source is None:
        raise SystemExit(
            "No se pudo crear el calendario EDS " + EDS_UID
        )

    return ECal.Client.connect_sync(
        source,
        ECal.ClientSourceType.EVENTS,
        10,
        None,
    )


def existing_components(client, uid):
    try:
        ok, objects = client.get_objects_for_uid_sync(
            uid,
            None,
        )
    except Exception:
        return {}

    if not ok:
        return {}

    result = {}

    for comp in objects:
        rid = comp.get_recurid_as_string() or ""
        result[rid] = comp

    return result


def scan_source():
    groups = defaultdict(list)
    invalid = 0
    duplicate_components = 0
    seen = set()
    files = sorted(SOURCE_ROOT.rglob("*.ics"))

    for path in files:
        try:
            raw = path.read_text(
                encoding="utf-8",
                errors="replace",
            )
        except OSError:
            continue

        relative = path.relative_to(SOURCE_ROOT)

        for vevent in extract_vevents(raw):
            tagged = add_source_file(
                vevent,
                relative,
            )

            comp = ECal.Component.new_from_string(tagged)

            if comp is None:
                invalid += 1
                continue

            uid = comp.get_uid() or ""
            rid = comp.get_recurid_as_string() or ""

            if not uid:
                invalid += 1
                continue

            identity = (uid, rid)

            if identity in seen:
                duplicate_components += 1
                continue

            seen.add(identity)

            groups[uid].append(
                {
                    "rid": rid,
                    "component": comp,
                    "file": relative,
                }
            )

    return files, groups, invalid, duplicate_components


def main():
    parser = argparse.ArgumentParser()

    parser.add_argument(
        "--sync",
        action="store_true",
        help="Import missing calendar components into EDS",
    )

    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Print each imported/skipped component",
    )

    args = parser.parse_args()

    if not SOURCE_ROOT.exists():
        raise SystemExit(
            "No existe " + str(SOURCE_ROOT)
        )

    client = connect()

    (
        files,
        groups,
        invalid,
        duplicate_components,
    ) = scan_source()

    total = sum(
        len(items)
        for items in groups.values()
    )

    already = 0
    missing = 0
    imported = 0
    failures = 0

    for uid in sorted(groups):
        source_items = sorted(
            groups[uid],
            key=lambda item: (
                item["rid"] != "",
                item["rid"],
            ),
        )

        existing = existing_components(
            client,
            uid,
        )

        for item in source_items:
            rid = item["rid"]
            comp = item["component"]

            if rid in existing:
                already += 1

                if args.verbose:
                    print(
                        "EXISTS",
                        uid,
                        rid or "MASTER",
                    )

                continue

            missing += 1

            if not args.sync:
                continue

            try:
                if rid == "":
                    result = client.create_object_sync(
                        comp.get_icalcomponent(),
                        ECal.OperationFlags.NONE,
                        None,
                    )
                else:
                    result = client.modify_object_sync(
                        comp.get_icalcomponent(),
                        ECal.ObjModType.THIS,
                        ECal.OperationFlags.NONE,
                        None,
                    )

                imported += 1
                existing[rid] = comp

                if args.verbose:
                    print(
                        "IMPORTED",
                        uid,
                        rid or "MASTER",
                        "|",
                        item["file"],
                        "|",
                        result,
                    )

            except Exception as error:
                failures += 1

                print(
                    "ERROR",
                    uid,
                    rid or "MASTER",
                    "|",
                    item["file"],
                    "|",
                    type(error).__name__,
                    str(error),
                )

    print()
    print("ICS files:", len(files))
    print("UID groups:", len(groups))
    print("Unique components:", total)
    print("Already in EDS:", already)
    print("Missing before sync:", missing)
    print("Imported:", imported)
    print("Invalid:", invalid)
    print("Duplicate identities:", duplicate_components)
    print("Failures:", failures)

    if not args.sync:
        print()
        print("Dry-run only: no events were modified.")

    if args.sync and failures:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
