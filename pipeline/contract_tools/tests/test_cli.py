from __future__ import annotations

import pytest
from fleur_contracts.cli import main


def test_cli_version_reports_package_version(capsys: pytest.CaptureFixture[str]) -> None:
    with pytest.raises(SystemExit) as exc_info:
        main(["--version"])

    assert exc_info.value.code == 0
    assert capsys.readouterr().out == "fleur-contracts 0.1.0\n"
