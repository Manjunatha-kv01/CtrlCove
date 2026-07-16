import tempfile
import unittest
from pathlib import Path

from login import (
    AuthService,
    AuthenticationError,
    UserAlreadyExistsError,
    ValidationError,
)


class AuthServiceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.service = AuthService(
            Path(self.temp_dir.name) / "auth.db",
            max_failed_attempts=2,
            lockout_seconds=60,
            session_ttl_seconds=3600,
        )

    def tearDown(self) -> None:
        self.service.close()
        self.temp_dir.cleanup()

    def test_register_authenticate_validate_and_logout(self) -> None:
        self.service.register("Admin.User", "correct horse battery staple")

        session = self.service.authenticate("admin.user", "correct horse battery staple")
        self.assertEqual(session.username, "admin.user")
        validated_session = self.service.validate_session(session.token)
        self.assertIsNotNone(validated_session)
        self.assertEqual(validated_session.username, session.username)
        raw_database = Path(self.temp_dir.name, "auth.db").read_bytes()
        self.assertNotIn(b"correct horse battery staple", raw_database)
        self.assertNotIn(session.token.encode("utf-8"), raw_database)

        self.service.logout(session.token)
        self.assertIsNone(self.service.validate_session(session.token))

    def test_duplicate_users_and_invalid_passwords_are_rejected(self) -> None:
        self.service.register("operator", "correct horse battery staple")

        with self.assertRaises(UserAlreadyExistsError):
            self.service.register("OPERATOR", "another correct password")
        with self.assertRaises(AuthenticationError):
            self.service.authenticate("operator", "wrong password value")

    def test_failed_attempts_temporarily_lock_account(self) -> None:
        self.service.register("operator", "correct horse battery staple")

        with self.assertRaises(AuthenticationError):
            self.service.authenticate("operator", "wrong password value")
        with self.assertRaises(AuthenticationError) as error:
            self.service.authenticate("operator", "wrong password value")
        self.assertIn("locked", str(error.exception).lower())

        with self.assertRaises(AuthenticationError):
            self.service.authenticate("operator", "correct horse battery staple")

    def test_password_policy_is_enforced(self) -> None:
        with self.assertRaises(ValidationError):
            self.service.register("ab", "too-short")


if __name__ == "__main__":
    unittest.main()
