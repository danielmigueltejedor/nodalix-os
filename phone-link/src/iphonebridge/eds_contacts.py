"""Synchronize the iPhone PBAP phonebook into Evolution Data Server.

GNOME Contacts reads address books from Evolution Data Server (EDS).  Keep the
phone's contacts in a dedicated local EDS source so Nodalix never rewrites the
user's other address books.  The iPhone remains authoritative; each PBAP refresh
updates this source in place.
"""
from __future__ import annotations

import hashlib
import logging
import re
from dataclasses import dataclass
import unicodedata

from iphonebridge.events import normalize_phone

log = logging.getLogger(__name__)

SOURCE_UID = "nodalix-iphone-contacts"
SOURCE_NAME = "iPhone (Nodalix)"


@dataclass(frozen=True, slots=True)
class DialContact:
    name: str
    number: str
    label: str = ""


def search_text(value: str) -> str:
    return ''.join(c for c in unicodedata.normalize('NFKD',value.casefold()) if not unicodedata.combining(c))


def dial_number(value: str) -> str | None:
    """Retain international prefixes without turning a local number into one."""
    raw=value.strip().removeprefix('tel:')
    if not re.fullmatch(r'\+?[\d\s().-]+',raw):return None
    digits=normalize_phone(raw)
    return ('+' if raw.startswith('+') else '')+digits if digits else None


def vcard_value(contact,name):
    attribute=contact.get_attribute(name)
    values=attribute.get_values() if attribute else []
    return str(values[0]) if values else ''


def contact_numbers(contact) -> list[DialContact]:
    # e_contact_get() is an untyped gpointer in GI, not a Python string.
    name=vcard_value(contact,'FN') or vcard_value(contact,'X-EVOLUTION-FILE-AS') or 'Sin nombre'
    result=[]
    labels={'CELL':'Móvil','HOME':'Casa','WORK':'Trabajo','MAIN':'Principal'}
    for attribute in contact.get_attributes():
        if attribute.get_name().upper()!='TEL':continue
        types=[str(t).upper() for t in attribute.get_param('TYPE') or []]
        label=next((labels[t] for t in types if t in labels),'Teléfono')
        for value in attribute.get_values():
            number=dial_number(str(value))
            if number:result.append(DialContact(name,number,label))
    return result


def read_gnome_contacts(cancellable=None) -> list[DialContact]:
    """Read all enabled GNOME address books, without modifying any contact."""
    import gi
    gi.require_version('EDataServer','1.2');gi.require_version('EBook','1.2');gi.require_version('EBookContacts','1.2')
    from gi.repository import EBook,EBookContacts,EDataServer,GLib
    registry=EDataServer.SourceRegistry.new_sync(cancellable)
    result={};read=0;failed=0
    for source in registry.list_sources(EDataServer.SOURCE_EXTENSION_ADDRESS_BOOK):
        if not registry.check_enabled(source):continue
        if cancellable and cancellable.is_cancelled():break
        try:
            client=EBook.BookClient.connect_sync(source,10,cancellable)
            contacts=_sync_payload(client.get_contacts_sync('#t',cancellable)) or []
            read+=1
            for contact in contacts:
                for entry in contact_numbers(contact):
                    result.setdefault((search_text(entry.name),normalize_phone(entry.number)),entry)
        except GLib.Error:
            if cancellable and cancellable.is_cancelled():break
            failed+=1
            log.warning('No se pudo consultar una libreta de Contactos de GNOME')
    if failed and not read:raise RuntimeError('No se han podido leer las libretas de Contactos de GNOME')
    return sorted(result.values(),key=lambda c:(search_text(c.name),c.number))

_VCARD_RE = re.compile(
    r"BEGIN:VCARD.*?END:VCARD",
    re.IGNORECASE | re.DOTALL,
)


def split_vcards(blob: str) -> list[str]:
    """Return complete vCard records from a PBAP payload."""
    cards: list[str] = []
    for match in _VCARD_RE.finditer(blob or ""):
        card = match.group(0).strip().replace("\r\n", "\n").replace("\r", "\n")
        cards.append(card.replace("\n", "\r\n") + "\r\n")
    return cards


def _field(card: str, name: str) -> str:
    prefix = name.upper() + ":"
    for raw in card.replace("\r\n", "\n").split("\n"):
        line = raw.strip()
        if line.upper().startswith(prefix):
            return line.split(":", 1)[1].strip()
    return ""


def _phones(card: str) -> list[str]:
    out: list[str] = []
    for raw in card.replace("\r\n", "\n").split("\n"):
        line = raw.strip()
        if not line.upper().startswith("TEL"):
            continue
        _, sep, value = line.partition(":")
        if not sep:
            continue
        norm = normalize_phone(value)
        if norm and norm not in out:
            out.append(norm)
    return sorted(out)


def stable_uid(card: str) -> str:
    """Stable EDS UID based on the contact identity we can trust from PBAP."""
    name = _field(card, "FN").casefold()
    phones = _phones(card)
    basis = "\0".join([name, *phones]).strip("\0")
    if not basis:
        basis = card.replace("\r\n", "\n").strip()
    digest = hashlib.sha256(basis.encode("utf-8", errors="replace")).hexdigest()
    return "nodalix-iphone-" + digest[:32]


def _sync_payload(value):
    """Unwrap GI synchronous calls that return (success, payload)."""
    if isinstance(value, tuple):
        if value and isinstance(value[0], bool):
            if not value[0]:
                raise RuntimeError("Evolution Data Server operation failed")
            return value[1] if len(value) > 1 else None
    return value


def sync_gnome_contacts(blob: str) -> int:
    """Mirror PBAP vCards into the dedicated EDS address book.

    Failures are raised to the caller so it can log them, but the Phone Link
    SQLite contact cache remains independent and usable.
    """
    try:
        import gi
        gi.require_version("EDataServer", "1.2")
        gi.require_version("EBook", "1.2")
        gi.require_version("EBookContacts", "1.2")
        from gi.repository import EBook, EBookContacts, EDataServer
    except (ImportError, ValueError) as error:
        raise RuntimeError(
            "Evolution Data Server contact bindings are unavailable"
        ) from error

    cards = split_vcards(blob)
    registry = EDataServer.SourceRegistry.new_sync(None)
    source = registry.ref_source(SOURCE_UID)

    if source is None:
        source = EDataServer.Source.new_with_uid(SOURCE_UID, None)
        source.set_display_name(SOURCE_NAME)
        source.set_enabled(True)
        extension = source.get_extension(
            EDataServer.SOURCE_EXTENSION_ADDRESS_BOOK
        )
        extension.set_backend_name("local")
        registry.commit_source_sync(source, None)
        source = registry.ref_source(SOURCE_UID)

    if source is None:
        raise RuntimeError("could not create iPhone EDS address book")

    client = EBook.BookClient.connect_sync(source, 10, None)
    existing = _sync_payload(client.get_contacts_sync("#t", None)) or []

    existing_by_uid = {}
    for contact in existing:
        uid = vcard_value(contact,'UID')
        if uid:
            existing_by_uid[str(uid)] = contact

    flags = EBookContacts.BookOperationFlags.NONE
    target_uids: set[str] = set()

    for card in cards:
        uid = stable_uid(card)
        target_uids.add(uid)
        contact = EBookContacts.Contact.new_from_vcard_with_uid(card, uid)
        if contact is None:
            log.warning("EDS skipped malformed iPhone vCard uid=%s", uid)
            continue

        if uid in existing_by_uid:
            _sync_payload(client.modify_contact_sync(contact, flags, None))
        else:
            _sync_payload(client.add_contact_sync(contact, flags, None))

    for uid, contact in existing_by_uid.items():
        if uid not in target_uids:
            _sync_payload(client.remove_contact_sync(contact, flags, None))

    log.info(
        "GNOME Contacts synchronized: source=%s contacts=%d",
        SOURCE_NAME,
        len(target_uids),
    )
    return len(target_uids)
