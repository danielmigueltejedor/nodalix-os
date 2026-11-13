"""Nodalix Package Manager.

The public Python API and the D-Bus API share one engine. Version 1 of the
D-Bus interface is ``os.nodalix.Nxpm1``. API 2 added ``ListCatalog`` and the
``nodalia`` source; nothing from API 1 changed.
"""

API_VERSION = 2

__all__ = ["API_VERSION"]
