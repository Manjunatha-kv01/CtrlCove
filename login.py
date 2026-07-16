"""Secure local authentication primitives for CYMOS.

The module deliberately keeps authentication local and dependency-free. Passwords and
session tokens are never stored in plaintext, and authentication failures return the same
public error regardless of whether a username exists.
"""

from __future__ import annotations

import hashlib
import hmac
import re
import secrets
import sqlite3
import threading
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Optional


class AuthenticationError(Exception):
    """Raised when credentials are invalid or the account is temporarily locked."""


class ValidationError(ValueError):
    """Raised when a registration or login input violates the contract."""


class UserAlreadyExistsError(Exception):
    """Raised when registration targets an existing username."""


@dataclass(frozen=True)
class LoginSession:
    """A session token returned only to the authenticated caller."""

    token: str
    username: str
    expires_at: int


class AuthService:
    """Local SQLite authentication service.

    The database stores only password-derived values and a SHA-256 digest of each
    session token. Callers must keep the returned raw token private.
    """

    _USERNAME_PATTERN = re.compile(r"^[a-z0-9][a-z0-9_.-]{2,63}$")
    _PASSWORD_MIN_LENGTH = 12
    _PASSWORD_MAX_LENGTH = 256
    _SCRYPT_N = 2**14
    _SCRYPT_R = 8
    _SCRYPT_P = 1
    _SALT_BYTES = 16
    _KEY_BYTES = 32
    _DUMMY_SALT = b"cymos-dummy-salt"

    def __init__(
        self,
        database_path: str | Path = "cymos-auth.db",
        *,
        max_failed_attempts: int = 5,
        lockout_seconds: int = 60,
        session_ttl_seconds: int = 3600,
    ) -> None:
        if max_failed_attempts < 1:
            raise ValidationError("max_failed_attempts must be positive")
        if lockout_seconds < 1 or session_ttl_seconds < 1:
            raise ValidationError("lockout and session durations must be positive")

        resolved_path = Path(database_path)
        if str(resolved_path) != ":memory:":
            resolved_path.parent.mkdir(parents=True, exist_ok=True)
        self._max_failed_attempts = max_failed_attempts
        self._lockout_seconds = lockout_seconds
        self._session_ttl_seconds = session_ttl_seconds
        self._lock = threading.RLock()
        self._connection = sqlite3.connect(str(resolved_path), timeout=5, check_same_thread=False)
        self._connection.row_factory = sqlite3.Row
        self._configure_database()

    def close(self) -> None:
        with self._lock:
            self._connection.close()

    def register(self, username: str, password: str) -> None:
        """Create a user with a unique normalized username."""

        normalized_username = self._validate_username(username)
        self._validate_password(password)
        salt = secrets.token_bytes(self._SALT_BYTES)
        password_hash = self._derive_password_hash(password, salt)
        now = int(time.time())

        with self._lock, self._connection:
            try:
                self._connection.execute(
                    """
                    INSERT INTO users (username, password_hash, password_salt, created_at, updated_at)
                    VALUES (?, ?, ?, ?, ?)
                    """,
                    (normalized_username, password_hash, salt, now, now),
                )
            except sqlite3.IntegrityError as error:
                raise UserAlreadyExistsError("Username is already registered") from error

    def authenticate(self, username: str, password: str) -> LoginSession:
        """Authenticate a user and issue a short-lived opaque session token."""

        normalized_username = self._validate_username(username)
        self._validate_password(password)
        now = int(time.time())

        with self._lock, self._connection:
            user = self._connection.execute(
                """
                SELECT id, username, password_hash, password_salt, failed_attempts, locked_until
                FROM users
                WHERE username = ?
                """,
                (normalized_username,),
            ).fetchone()

            if user is None:
                self._derive_password_hash(password, self._DUMMY_SALT)
                raise AuthenticationError("Invalid username or password")

            if user["locked_until"] and user["locked_until"] > now:
                raise AuthenticationError("Account is temporarily locked")

            candidate_hash = self._derive_password_hash(password, user["password_salt"])
            if not hmac.compare_digest(candidate_hash, user["password_hash"]):
                self._record_failed_attempt(user["id"], user["failed_attempts"], now)
                self._connection.commit()
                if user["failed_attempts"] + 1 >= self._max_failed_attempts:
                    raise AuthenticationError("Account is temporarily locked")
                raise AuthenticationError("Invalid username or password")

            self._connection.execute(
                """
                UPDATE users
                SET failed_attempts = 0, locked_until = NULL, last_login_at = ?, updated_at = ?
                WHERE id = ?
                """,
                (now, now, user["id"]),
            )
            raw_token = secrets.token_urlsafe(32)
            expires_at = now + self._session_ttl_seconds
            self._connection.execute(
                """
                INSERT INTO sessions (user_id, token_hash, created_at, expires_at)
                VALUES (?, ?, ?, ?)
                """,
                (user["id"], self._token_digest(raw_token), now, expires_at),
            )

            return LoginSession(raw_token, user["username"], expires_at)

    def validate_session(self, token: str) -> Optional[LoginSession]:
        """Return the active session identity, or ``None`` for an invalid token."""

        if not isinstance(token, str) or not token:
            return None

        now = int(time.time())
        with self._lock, self._connection:
            session = self._connection.execute(
                """
                SELECT users.username, sessions.expires_at
                FROM sessions
                JOIN users ON users.id = sessions.user_id
                WHERE sessions.token_hash = ? AND sessions.revoked_at IS NULL AND sessions.expires_at > ?
                """,
                (self._token_digest(token), now),
            ).fetchone()
            if session is None:
                return None
            return LoginSession(token, session["username"], session["expires_at"])

    def logout(self, token: str) -> None:
        """Revoke a session token without exposing whether it existed."""

        if not isinstance(token, str) or not token:
            return
        with self._lock, self._connection:
            self._connection.execute(
                "UPDATE sessions SET revoked_at = ? WHERE token_hash = ? AND revoked_at IS NULL",
                (int(time.time()), self._token_digest(token)),
            )

    def cleanup_expired_sessions(self) -> int:
        """Delete expired or revoked session records and return the deleted count."""

        now = int(time.time())
        with self._lock, self._connection:
            result = self._connection.execute(
                "DELETE FROM sessions WHERE expires_at <= ? OR revoked_at IS NOT NULL",
                (now,),
            )
            return result.rowcount

    def _configure_database(self) -> None:
        with self._connection:
            self._connection.execute("PRAGMA foreign_keys = ON")
            self._connection.execute("PRAGMA journal_mode = WAL")
            self._connection.execute("PRAGMA synchronous = NORMAL")
            self._connection.executescript(
                """
                CREATE TABLE IF NOT EXISTS users (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    username TEXT NOT NULL UNIQUE,
                    password_hash BLOB NOT NULL,
                    password_salt BLOB NOT NULL,
                    failed_attempts INTEGER NOT NULL DEFAULT 0,
                    locked_until INTEGER,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL,
                    last_login_at INTEGER
                );

                CREATE TABLE IF NOT EXISTS sessions (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                    token_hash BLOB NOT NULL UNIQUE,
                    created_at INTEGER NOT NULL,
                    expires_at INTEGER NOT NULL,
                    revoked_at INTEGER
                );

                CREATE INDEX IF NOT EXISTS idx_sessions_expiry ON sessions(expires_at);
                CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
                """
            )

    def _record_failed_attempt(self, user_id: int, failed_attempts: int, now: int) -> None:
        attempts = failed_attempts + 1
        locked_until = now + self._lockout_seconds if attempts >= self._max_failed_attempts else None
        if locked_until is not None:
            attempts = 0
        self._connection.execute(
            "UPDATE users SET failed_attempts = ?, locked_until = ?, updated_at = ? WHERE id = ?",
            (attempts, locked_until, now, user_id),
        )

    @classmethod
    def _validate_username(cls, username: str) -> str:
        if not isinstance(username, str):
            raise ValidationError("Username must be text")
        normalized = username.strip().casefold()
        if not cls._USERNAME_PATTERN.fullmatch(normalized):
            raise ValidationError("Username must be 3-64 characters using letters, numbers, '.', '_' or '-'")
        return normalized

    @classmethod
    def _validate_password(cls, password: str) -> None:
        if not isinstance(password, str):
            raise ValidationError("Password must be text")
        if not cls._PASSWORD_MIN_LENGTH <= len(password) <= cls._PASSWORD_MAX_LENGTH:
            raise ValidationError("Password must be 12-256 characters")

    @classmethod
    def _derive_password_hash(cls, password: str, salt: bytes) -> bytes:
        return hashlib.scrypt(
            password.encode("utf-8"),
            salt=salt,
            n=cls._SCRYPT_N,
            r=cls._SCRYPT_R,
            p=cls._SCRYPT_P,
            dklen=cls._KEY_BYTES,
        )

    @staticmethod
    def _token_digest(token: str) -> bytes:
        return hashlib.sha256(token.encode("utf-8")).digest()
