"""Stable error codes shared by the library, the CLI and D-Bus."""


class NxpmError(Exception):
    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code
        self.message = message

    @property
    def dbus_name(self) -> str:
        suffix = "".join(part.capitalize() for part in self.code.split("-"))
        return f"os.nodalix.Nxpm1.Error.{suffix}"
