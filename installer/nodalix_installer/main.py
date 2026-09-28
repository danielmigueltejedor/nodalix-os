from pathlib import Path
import sys

from PySide6.QtCore import QUrl
from PySide6.QtGui import QGuiApplication, QIcon
from PySide6.QtQml import QQmlApplicationEngine

from .backend import InstallerBackend


def main():
    app = QGuiApplication(sys.argv)
    app.setApplicationName("Nodalix Installer")
    app.setOrganizationName("Nodalix")
    app.setWindowIcon(QIcon(str(Path(__file__).parent / "assets" / "nodalix.png")))

    engine = QQmlApplicationEngine()

    backend = InstallerBackend(engine)
    engine.rootContext().setContextProperty("installer", backend)

    qml = Path(__file__).parent / "ui" / "Main.qml"
    engine.load(QUrl.fromLocalFile(str(qml)))

    if not engine.rootObjects():
        return 1

    return app.exec()


if __name__ == "__main__":
    raise SystemExit(main())
