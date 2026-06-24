from __future__ import annotations

import json
import os
import urllib.error
import urllib.request
from typing import Any

import dagster as dg


class RearviewApiResource(dg.ConfigurableResource):
    """HTTP client for Rearview control-plane APIs."""

    base_url: str = ""
    timeout_seconds: int = 30

    def create_strategy_portfolio_daily_runs(
        self,
        *,
        trade_date: str,
        client_request_id: str,
    ) -> dict[str, Any]:
        return self._post_json(
            "/rearview/strategy-portfolios/daily-runs",
            {
                "trade_date": trade_date,
                "client_request_id": client_request_id,
            },
        )

    def _post_json(self, path: str, payload: dict[str, Any]) -> dict[str, Any]:
        url = f"{self._resolved_base_url()}{path}"
        body = json.dumps(payload).encode("utf-8")
        request = urllib.request.Request(
            url,
            data=body,
            headers={"Content-Type": "application/json", "Accept": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(request, timeout=self.timeout_seconds) as response:
                response_body = response.read().decode("utf-8")
        except urllib.error.HTTPError as error:
            response_body = error.read().decode("utf-8", errors="replace")
            msg = f"Rearview API POST {path} failed with HTTP {error.code}: {response_body}"
            raise RuntimeError(msg) from error
        except urllib.error.URLError as error:
            msg = f"Rearview API POST {path} failed: {error.reason}"
            raise RuntimeError(msg) from error

        if response_body.strip() == "":
            return {}
        parsed = json.loads(response_body)
        if not isinstance(parsed, dict):
            msg = f"Rearview API POST {path} returned non-object JSON"
            raise RuntimeError(msg)
        return parsed

    def _resolved_base_url(self) -> str:
        base_url = (
            self.base_url
            or os.getenv("REARVIEW_API_BASE_URL")
            or os.getenv("VITE_REARVIEW_API_BASE_URL")
            or "http://127.0.0.1:34057"
        )
        return base_url.rstrip("/")
