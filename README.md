# CtrlCove Login

This branch adds a dependency-free local authentication service for CYMOS.

## Usage

```python
from login import AuthService

auth = AuthService("cymos-auth.db")
auth.register("operator", "a long passphrase of at least 12 characters")

session = auth.authenticate("operator", "a long passphrase of at least 12 characters")
current_user = auth.validate_session(session.token)
auth.logout(session.token)
auth.close()
```

## Security behavior

- Passwords are derived with `hashlib.scrypt`; plaintext passwords are never stored.
- Session tokens are generated with `secrets` and only their SHA-256 digests are stored.
- Failed logins trigger temporary account lockout after the configured threshold.
- Usernames are normalized with `casefold()` and constrained to a safe identifier format.
- SQLite foreign keys, WAL mode, and indexes are enabled during initialization.
- The local authentication database is ignored by Git and must be protected by the host operating system.

Run the tests with:

```bash
python3 -m unittest -v
```
