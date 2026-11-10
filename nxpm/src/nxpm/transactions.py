"""One transaction queue. Pending work can be cancelled; running work usually cannot."""

from __future__ import annotations

import logging
import threading
import uuid
from collections.abc import Callable

from nxpm.errors import NxpmError
from nxpm.model import TransactionRecord

log = logging.getLogger(__name__)
Runner = Callable[["Transaction", "TransactionManager"], None]
TERMINAL = {"succeeded", "failed", "cancelled"}


class Transaction:
    def __init__(self, operation: str, package_id: str, source: str) -> None:
        self.id = uuid.uuid4().hex
        self.operation = operation
        self.package_id = package_id
        self.source = source
        self.state = "pending"
        self.progress = 0.0
        self.message = "Waiting"
        self.error = ""
        self.cancellable = True
        self.cancel_requested = False

    def record(self) -> TransactionRecord:
        return TransactionRecord(
            id=self.id,
            operation=self.operation,
            package_id=self.package_id,
            source=self.source,
            state=self.state,
            progress=self.progress,
            message=self.message,
            error=self.error,
            cancellable=self.cancellable and self.state == "pending",
        )


class TransactionManager:
    def __init__(self) -> None:
        self._items: dict[str, Transaction] = {}
        self._queue: list[tuple[Transaction, Runner]] = []
        self._active: str | None = None
        self._listeners: list[Callable[[Transaction], None]] = []
        self._lock = threading.Lock()
        self._history: list[dict] = []
        self.defer = False

    def add_listener(self, listener: Callable[[Transaction], None]) -> None:
        self._listeners.append(listener)

    def submit(self, operation: str, package_id: str, source: str, runner: Runner) -> Transaction:
        transaction = Transaction(operation, package_id, source)
        with self._lock:
            self._items[transaction.id] = transaction
            self._queue.append((transaction, runner))
        self._emit(transaction)
        if self.defer:
            from gi.repository import GLib

            # Keep the GLib/D-Bus loop responsive: run the transaction body on a
            # worker thread and only marshal status updates back onto the loop.
            def _start_worker() -> bool:
                threading.Thread(target=self.pump, name="nxpm-transaction", daemon=True).start()
                return False

            GLib.idle_add(_start_worker)
        else:
            self.pump()
        return transaction

    def busy(self) -> bool:
        with self._lock:
            return self._active is not None or bool(self._queue)

    def get(self, transaction_id: str) -> Transaction:
        with self._lock:
            transaction = self._items.get(transaction_id)
        if transaction is None:
            raise NxpmError("not-found", "That operation does not exist.")
        return transaction

    def cancel(self, transaction_id: str) -> bool:
        with self._lock:
            transaction = self._items.get(transaction_id)
            if transaction is None:
                raise NxpmError("not-found", "That operation does not exist.")
            if transaction.state == "running":
                transaction.message = "This operation can no longer be cancelled safely."
                cancellable = False
            elif transaction.state != "pending":
                cancellable = False
            else:
                transaction.cancel_requested = True
                transaction.state = "cancelled"
                transaction.cancellable = False
                transaction.message = "Cancelled before it started."
                self._queue = [item for item in self._queue if item[0].id != transaction.id]
                cancellable = True
        self._emit(transaction)
        if cancellable:
            self._remember(transaction)
        return cancellable

    def history(self, limit: int = 50) -> list[dict]:
        if limit < 0:
            limit = 0
        with self._lock:
            return list(self._history[:limit])

    def pump(self) -> None:
        with self._lock:
            if self._active is not None or not self._queue:
                return
            transaction, runner = self._queue.pop(0)
            if transaction.state == "cancelled":
                should_continue = True
                runner = None
            else:
                self._active = transaction.id
                transaction.state = "running"
                transaction.cancellable = False
                transaction.message = "Running"
                should_continue = False
        if runner is None:
            if should_continue:
                self.pump()
            return
        self._emit(transaction)
        try:
            runner(transaction, self)
        except Exception as exc:
            transaction.state = "failed"
            transaction.error = str(exc)
            transaction.message = str(exc)
        else:
            if transaction.cancel_requested and transaction.state == "running":
                transaction.state = "cancelled"
                transaction.message = "Cancelled."
            elif transaction.state == "running":
                transaction.state = "succeeded"
                transaction.progress = 1.0
                transaction.message = transaction.message or "Finished"
        finally:
            # Stay busy until the result is recorded: an idle service must not
            # exit between finishing a transaction and remembering it.
            self._emit(transaction)
            self._remember(transaction)
            with self._lock:
                self._active = None
            self.pump()

    def publish(self, transaction: Transaction) -> None:
        self._emit(transaction)

    def _emit(self, transaction: Transaction) -> None:
        listeners = list(self._listeners)
        # Capture state now: deferred GLib callbacks must not observe later
        # mutations from the worker thread as if they were earlier updates.
        snapshot = transaction.record()

        def notify() -> bool:
            for listener in listeners:
                try:
                    listener(snapshot)
                except Exception:
                    log.exception("Transaction listener failed")
            return False

        if self.defer and threading.current_thread() is not threading.main_thread():
            from gi.repository import GLib

            GLib.idle_add(notify)
        else:
            notify()

    def _remember(self, transaction: Transaction) -> None:
        if transaction.state not in TERMINAL:
            return
        entry = transaction.record().to_dict()
        with self._lock:
            self._history.insert(0, entry)
            del self._history[200:]
