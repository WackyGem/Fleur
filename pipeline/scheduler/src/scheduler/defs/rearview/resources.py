from __future__ import annotations

import json
import os
import urllib.error
import urllib.parse
import urllib.request
from typing import Any

import dagster as dg


class RearviewApiResource(dg.ConfigurableResource):
    """HTTP client for Rearview control-plane APIs."""

    base_url: str = ""
    timeout_seconds: int = 30

    def ensure_racingline_0051_low_reversal_portfolio(self) -> dict[str, Any]:
        return self._post_json(
            "/rearview/examples/strategy-portfolios/racingline-0051-low-reversal/ensure",
            {},
        )

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

    def create_strategy_portfolio_daily_runs_range(
        self,
        *,
        start_date: str,
        end_date: str,
        client_request_id: str,
        max_trade_dates: int,
        strategy_portfolio_id: str = "",
    ) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "start_date": start_date,
            "end_date": end_date,
            "client_request_id": client_request_id,
            "max_trade_dates": max_trade_dates,
        }
        if strategy_portfolio_id.strip():
            payload["strategy_portfolio_id"] = strategy_portfolio_id.strip()
        return self._post_json(
            "/rearview/strategy-portfolios/daily-runs/range",
            payload,
        )

    def get_strategy_portfolio_daily_run_status(self, daily_run_id: str) -> dict[str, Any]:
        return self._get_json(f"/rearview/strategy-portfolios/daily-runs/{daily_run_id}")

    def get_strategy_portfolio_daily_run_fact_counts(self, daily_run_id: str) -> dict[str, Any]:
        return self._get_json(
            f"/rearview/strategy-portfolios/daily-runs/{daily_run_id}/fact-counts"
        )

    def get_strategy_portfolio_settlement_target(
        self,
        *,
        strategy_portfolio_id: str = "",
    ) -> dict[str, Any]:
        path = "/rearview/strategy-portfolios/daily-runs/settlement-target"
        if strategy_portfolio_id.strip():
            query = urllib.parse.urlencode({"strategy_portfolio_id": strategy_portfolio_id.strip()})
            path = f"{path}?{query}"
        return self._get_json(path)

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

    def _get_json(self, path: str) -> dict[str, Any]:
        url = f"{self._resolved_base_url()}{path}"
        request = urllib.request.Request(
            url,
            headers={"Accept": "application/json"},
            method="GET",
        )
        try:
            with urllib.request.urlopen(request, timeout=self.timeout_seconds) as response:
                response_body = response.read().decode("utf-8")
        except urllib.error.HTTPError as error:
            response_body = error.read().decode("utf-8", errors="replace")
            msg = f"Rearview API GET {path} failed with HTTP {error.code}: {response_body}"
            raise RuntimeError(msg) from error
        except urllib.error.URLError as error:
            msg = f"Rearview API GET {path} failed: {error.reason}"
            raise RuntimeError(msg) from error

        if response_body.strip() == "":
            return {}
        parsed = json.loads(response_body)
        if not isinstance(parsed, dict):
            msg = f"Rearview API GET {path} returned non-object JSON"
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
