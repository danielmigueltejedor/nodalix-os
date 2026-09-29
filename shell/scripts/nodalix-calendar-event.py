#!/usr/bin/env python3

import argparse
import json
import re
from pathlib import Path

import gi

gi.require_version("ECal", "2.0")
gi.require_version("EDataServer", "1.2")

from gi.repository import ECal, EDataServer

EDS_UID = "nodalix-calendar"
STATE = Path.home() / ".local/state/nodalix/calendar-bridge.json"


def connect():
    registry = EDataServer.SourceRegistry.new_sync(None)
    source = registry.ref_source(EDS_UID)
    if source is None:
        raise SystemExit("Nodalix EDS calendar is not available")
    return ECal.Client.connect_sync(
        source,
        ECal.ClientSourceType.EVENTS,
        10,
        None,
    )


def assert_writable(uid):
    if not STATE.exists():
        raise SystemExit("Calendar bridge state is not initialized")
    data = json.loads(STATE.read_text(encoding="utf-8"))
    item = data.get("uids", {}).get(uid)
    if not item or not item.get("writable"):
        raise SystemExit("Event is not writable through Nodalix")


def ical_text(component):
    return component.get_icalcomponent().as_ical_string()


def unfold(text):
    out = []
    for line in text.replace("\r\n", "\n").replace("\r", "\n").split("\n"):
        if line.startswith((" ", "\t")) and out:
            out[-1] += line[1:]
        else:
            out.append(line)
    return out


def prop_line(text, name):
    prefix = name.upper()
    for line in unfold(text):
        left = line.split(":", 1)[0]
        if left.split(";", 1)[0].upper() == prefix:
            return line
    return ""


def prop_value(text, name):
    line = prop_line(text, name)
    return line.split(":", 1)[1] if ":" in line else ""


def escape_text(value):
    return (
        str(value or "")
        .replace("\\", "\\\\")
        .replace("\n", "\\n")
        .replace(";", "\\;")
        .replace(",", "\\,")
    )


def datetime_value(date_value, time_value, template_line):
    date_digits = str(date_value or "").replace("-", "")
    time_digits = str(time_value or "").replace(":", "")
    if not date_digits:
        raise ValueError("Missing date")

    left = template_line.split(":", 1)[0] if ":" in template_line else "DTSTART"
    original = template_line.split(":", 1)[1] if ":" in template_line else ""

    if "VALUE=DATE" in left.upper() or (not time_digits and "T" not in original):
        return date_digits

    if not time_digits:
        if "T" in original:
            time_digits = original.split("T", 1)[1].rstrip("Z")[:6]
        else:
            raise ValueError("Missing time")

    if len(time_digits) == 4:
        time_digits += "00"

    suffix = "Z" if original.endswith("Z") and "TZID=" not in left.upper() else ""
    return date_digits + "T" + time_digits + suffix


def replace_property(text, name, value, template_left=None, remove_if_empty=False):
    lines = unfold(text)
    output = []
    replaced = False
    upper_name = name.upper()

    for line in lines:
        if not line:
            continue

        left = line.split(":", 1)[0]
        prop = left.split(";", 1)[0].upper()

        if prop != upper_name:
            output.append(line)
            continue

        if not replaced:
            if not (remove_if_empty and value == ""):
                output.append((template_left or left) + ":" + value)
            replaced = True

    if not replaced and not (remove_if_empty and value == ""):
        insert_at = len(output)
        for index, line in enumerate(output):
            if line == "END:VEVENT":
                insert_at = index
                break
        output.insert(insert_at, (template_left or name) + ":" + value)

    return "\r\n".join(output) + "\r\n"


def remove_properties(text, names):
    names = {name.upper() for name in names}
    lines = []
    for line in unfold(text):
        if not line:
            continue
        left = line.split(":", 1)[0]
        prop = left.split(";", 1)[0].upper()
        if prop not in names:
            lines.append(line)
    return "\r\n".join(lines) + "\r\n"


def start_key(text):
    line = prop_line(text, "DTSTART")
    value = line.split(":", 1)[1] if ":" in line else ""
    digits = value.rstrip("Z")
    if "T" in digits:
        return digits[:8], digits[9:13]
    return digits[:8], ""


def selected_key(date_value, time_value):
    return (
        str(date_value or "").replace("-", ""),
        str(time_value or "").replace(":", "")[:4],
    )


def objects_for_uid(client, uid):
    ok, objects = client.get_objects_for_uid_sync(uid, None)
    if not ok or not objects:
        raise SystemExit("Event not found in EDS: " + uid)

    master = None
    detached = []

    for component in objects:
        if component.get_recurid_as_string():
            detached.append(component)
        else:
            master = component

    if master is None:
        raise SystemExit("Recurring master not found for " + uid)

    return master, detached


def recurrence_left(master_text):
    start = prop_line(master_text, "DTSTART")
    left = start.split(":", 1)[0] if ":" in start else "DTSTART"
    params = left[len("DTSTART"):]
    return "RECURRENCE-ID" + params


def recurrence_value(master_text, date_value, time_value):
    return datetime_value(
        date_value,
        time_value,
        prop_line(master_text, "DTSTART"),
    )


def find_detached(detached, date_value, time_value):
    wanted = selected_key(date_value, time_value)
    for component in detached:
        if start_key(ical_text(component)) == wanted:
            return component
    return None


def updated_component(
    base_text,
    title,
    start_date,
    start_time,
    end_date,
    end_time,
    location,
):
    start_line = prop_line(base_text, "DTSTART")
    end_line = prop_line(base_text, "DTEND")

    new_start = datetime_value(start_date, start_time, start_line)
    new_end = datetime_value(
        end_date or start_date,
        end_time,
        end_line or start_line,
    )

    text = replace_property(
        base_text,
        "DTSTART",
        new_start,
        start_line.split(":", 1)[0] if ":" in start_line else "DTSTART",
    )
    text = replace_property(
        text,
        "DTEND",
        new_end,
        end_line.split(":", 1)[0] if ":" in end_line else (
            start_line.split(":", 1)[0].replace("DTSTART", "DTEND")
            if ":" in start_line
            else "DTEND"
        ),
    )
    text = replace_property(text, "SUMMARY", escape_text(title))
    text = replace_property(
        text,
        "LOCATION",
        escape_text(location),
        remove_if_empty=True,
    )
    return text


def make_detached(
    master_text,
    original_date,
    original_time,
    title,
    start_date,
    start_time,
    end_date,
    end_time,
    location,
):
    text = remove_properties(
        master_text,
        {
            "RRULE",
            "RDATE",
            "EXDATE",
            "EXRULE",
            "RECURRENCE-ID",
        },
    )

    rid_left = recurrence_left(master_text)
    rid_value = recurrence_value(master_text, original_date, original_time)
    text = replace_property(
        text,
        "RECURRENCE-ID",
        rid_value,
        rid_left,
    )

    return updated_component(
        text,
        title,
        start_date,
        start_time,
        end_date,
        end_time,
        location,
    )


def update(args):
    assert_writable(args.uid)
    client = connect()
    master, detached = objects_for_uid(client, args.uid)

    master_text = ical_text(master)
    recurring = master.has_recurrences()
    existing = find_detached(
        detached,
        args.original_date,
        args.original_start,
    )

    if recurring:
        if existing is not None:
            text = updated_component(
                ical_text(existing),
                args.title,
                args.start_date,
                args.start_time,
                args.end_date,
                args.end_time,
                args.location,
            )
        else:
            text = make_detached(
                master_text,
                args.original_date,
                args.original_start,
                args.title,
                args.start_date,
                args.start_time,
                args.end_date,
                args.end_time,
                args.location,
            )
        mode = ECal.ObjModType.THIS
    else:
        text = updated_component(
            master_text,
            args.title,
            args.start_date,
            args.start_time,
            args.end_date,
            args.end_time,
            args.location,
        )
        mode = ECal.ObjModType.THIS

    component = ECal.Component.new_from_string(text)
    if component is None:
        raise SystemExit("Could not build updated VEVENT")

    if args.dry_run:
        print(
            "DRY-RUN update",
            args.uid,
            component.get_recurid_as_string() or "MASTER",
        )
        return

    client.modify_object_sync(
        component.get_icalcomponent(),
        mode,
        ECal.OperationFlags.NONE,
        None,
    )

    print("updated", args.uid)


def delete(args):
    assert_writable(args.uid)
    client = connect()
    master, detached = objects_for_uid(client, args.uid)

    if master.has_recurrences():
        existing = find_detached(
            detached,
            args.original_date,
            args.original_start,
        )
        if existing is not None:
            rid = existing.get_recurid_as_string()
        else:
            rid = recurrence_value(
                ical_text(master),
                args.original_date,
                args.original_start,
            )

        if args.dry_run:
            print("DRY-RUN delete", args.uid, rid)
            return

        client.remove_object_sync(
            args.uid,
            rid,
            ECal.ObjModType.ONLY_THIS,
            ECal.OperationFlags.NONE,
            None,
        )
    else:
        if args.dry_run:
            print("DRY-RUN delete", args.uid, "ALL")
            return

        all_mode = getattr(ECal.ObjModType, "ALL", ECal.ObjModType(7))
        client.remove_object_sync(
            args.uid,
            None,
            all_mode,
            ECal.OperationFlags.NONE,
            None,
        )

    print("deleted", args.uid)


def main():
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)

    edit = sub.add_parser("update")
    edit.add_argument("--uid", required=True)
    edit.add_argument("--original-date", required=True)
    edit.add_argument("--original-start", default="")
    edit.add_argument("--start-date", required=True)
    edit.add_argument("--start-time", default="")
    edit.add_argument("--end-date", required=True)
    edit.add_argument("--end-time", default="")
    edit.add_argument("--title", required=True)
    edit.add_argument("--location", default="")
    edit.add_argument("--dry-run", action="store_true")
    edit.set_defaults(func=update)

    remove = sub.add_parser("delete")
    remove.add_argument("--uid", required=True)
    remove.add_argument("--original-date", required=True)
    remove.add_argument("--original-start", default="")
    remove.add_argument("--dry-run", action="store_true")
    remove.set_defaults(func=delete)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
