from __future__ import annotations

import asyncio
import unittest
from collections.abc import Mapping

import aiohttp

from scheduler.defs.http_resources.client import (
    AioHttpClient,
    HttpRequest,
    HttpRequestError,
    browser_json_headers,
    browser_text_headers,
    with_referer,
)
from scheduler.defs.util import ExponentialBackoffPolicy


class FakeAioHttpResponse:
    def __init__(
        self,
        *,
        status: int = 200,
        body: str = "",
        headers: Mapping[str, str] | None = None,
    ) -> None:
        self.status = status
        self._body = body
        self.headers = dict(headers or {})

    async def __aenter__(self) -> FakeAioHttpResponse:
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: object,
    ) -> None:
        return None

    async def text(self) -> str:
        return self._body


class FakeAioHttpSession:
    def __init__(self, outcomes: list[object]) -> None:
        self.outcomes = outcomes
        self.requests: list[dict[str, object]] = []
        self.closed = False

    def request(self, method: str, url: str, **kwargs: object) -> FakeAioHttpResponse:
        self.requests.append({"method": method, "url": url, **kwargs})
        outcome = self.outcomes.pop(0)
        if isinstance(outcome, BaseException):
            raise outcome
        if not isinstance(outcome, FakeAioHttpResponse):
            raise TypeError("Fake outcome must be a response or exception")
        return outcome

    async def close(self) -> None:
        self.closed = True


class AioHttpClientTest(unittest.IsolatedAsyncioTestCase):
    async def test_context_manager_closes_session(self) -> None:
        session = FakeAioHttpSession([FakeAioHttpResponse(body="ok")])

        async with AioHttpClient(session_factory=lambda: session) as client:
            response = await client.request_text(HttpRequest(method="GET", url="https://x"))

        self.assertEqual(response.body, "ok")
        self.assertTrue(session.closed)

    async def test_get_text_passes_params_and_headers(self) -> None:
        session = FakeAioHttpSession([FakeAioHttpResponse(body="ok")])

        async with AioHttpClient(
            headers=with_referer(browser_text_headers(), "https://referer.example/"),
            session_factory=lambda: session,
        ) as client:
            await client.request_text(
                HttpRequest(
                    method="GET",
                    url="https://example.test",
                    params={"a": "1"},
                )
            )

        request = session.requests[0]
        self.assertEqual(request["method"], "GET")
        self.assertEqual(request["params"], {"a": "1"})
        self.assertIn("User-Agent", request["headers"])
        self.assertEqual(request["headers"]["Referer"], "https://referer.example/")

    async def test_post_json_object_sends_json_body(self) -> None:
        session = FakeAioHttpSession([FakeAioHttpResponse(body='{"ok": true}')])

        async with AioHttpClient(
            headers=browser_json_headers(),
            session_factory=lambda: session,
        ) as client:
            payload = await client.request_json_object(
                HttpRequest(
                    method="POST",
                    url="https://example.test",
                    json_body={"hello": "world"},
                )
            )

        self.assertEqual(payload["ok"], True)
        self.assertEqual(session.requests[0]["json"], {"hello": "world"})

    async def test_http_429_and_5xx_retry(self) -> None:
        session = FakeAioHttpSession(
            [
                FakeAioHttpResponse(status=429, body="slow down"),
                FakeAioHttpResponse(status=500, body="bad gateway"),
                FakeAioHttpResponse(body='{"ok": 1}'),
            ]
        )

        async with AioHttpClient(
            retry_policy=ExponentialBackoffPolicy(jitter=False),
            session_factory=lambda: session,
        ) as client:
            payload = await client.request_json_object(
                HttpRequest(method="GET", url="https://example.test")
            )

        self.assertEqual(payload["ok"], 1)
        self.assertEqual(client.stats.request_count, 3)
        self.assertEqual(client.stats.retry_count, 2)
        self.assertEqual(client.stats.http_5xx_count, 1)

    async def test_http_4xx_does_not_retry(self) -> None:
        session = FakeAioHttpSession([FakeAioHttpResponse(status=404, body="nope")])

        async with AioHttpClient(session_factory=lambda: session) as client:
            with self.assertRaises(HttpRequestError):
                await client.request_text(HttpRequest(method="GET", url="https://example.test"))

        self.assertEqual(client.stats.request_count, 1)
        self.assertEqual(client.stats.retry_count, 0)
        self.assertEqual(client.stats.http_4xx_count, 1)

    async def test_client_error_and_json_decode_retry(self) -> None:
        session = FakeAioHttpSession(
            [
                aiohttp.ClientConnectionError("connection reset"),
                FakeAioHttpResponse(body="not json"),
                FakeAioHttpResponse(body='{"ok": "yes"}'),
            ]
        )

        async with AioHttpClient(
            retry_policy=ExponentialBackoffPolicy(jitter=False),
            session_factory=lambda: session,
        ) as client:
            payload = await client.request_json_object(
                HttpRequest(method="GET", url="https://example.test")
            )

        self.assertEqual(payload["ok"], "yes")
        self.assertEqual(client.stats.request_count, 3)
        self.assertEqual(client.stats.retry_count, 2)
        self.assertEqual(client.stats.transient_error_count, 1)
        self.assertEqual(client.stats.decode_error_count, 1)

    async def test_dynamic_header_factory_is_called_for_each_attempt(self) -> None:
        calls = 0

        def headers() -> dict[str, str]:
            nonlocal calls
            calls += 1
            return {"X-Request-Timestamp": str(calls)}

        session = FakeAioHttpSession(
            [
                FakeAioHttpResponse(status=500, body="temporary"),
                FakeAioHttpResponse(body="ok"),
            ]
        )

        async with AioHttpClient(
            headers=headers,
            retry_policy=ExponentialBackoffPolicy(jitter=False),
            session_factory=lambda: session,
        ) as client:
            await client.request_text(HttpRequest(method="GET", url="https://example.test"))

        self.assertEqual(calls, 2)
        self.assertEqual(session.requests[0]["headers"]["X-Request-Timestamp"], "1")
        self.assertEqual(session.requests[1]["headers"]["X-Request-Timestamp"], "2")


class EventLoopTest(unittest.TestCase):
    def test_asyncio_timeout_is_available_for_retry_scope(self) -> None:
        self.assertTrue(issubclass(asyncio.TimeoutError, TimeoutError))
