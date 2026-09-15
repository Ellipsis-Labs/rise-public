"""Run with python scripts/test_rust_publish.py."""

import contextlib
import io
from pathlib import Path

from rust_publish import Package, Workspace, internal_dependencies


def test_internal_dependency_versions():
    dependency = Package("math", "math", "0.5.1", Path("math/Cargo.toml"), True, [])
    workspace = Workspace(Path("."), {"math": dependency}, {"math": dependency})
    for requirement in ("=0.5.1", "^0.5.1", "^0.5.0", "=0.5.2", "*", ""):
        package = Package(
            "accounts", "accounts", "0.5.1", Path("accounts/Cargo.toml"), True,
            [{"name": "math", "req": requirement}],
        )
        with contextlib.redirect_stderr(io.StringIO()):
            try:
                result = internal_dependencies(workspace, package)
            except SystemExit as error:
                assert error.code == 1
                assert requirement not in ("=0.5.1", "^0.5.1"), requirement
            else:
                assert requirement in ("=0.5.1", "^0.5.1"), requirement
                assert result == {"math"}


if __name__ == "__main__":
    test_internal_dependency_versions()
