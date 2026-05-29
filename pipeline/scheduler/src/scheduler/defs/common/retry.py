from __future__ import annotations

import random
from collections.abc import Callable
from dataclasses import dataclass, field

RandomUniform = Callable[[float, float], float]


@dataclass(frozen=True)
class ExponentialBackoffPolicy:
    """Configurable exponential backoff schedule for transient remote failures."""

    base_delay: float = 1.0
    factor: float = 2.0
    max_delay: float = 60.0
    jitter: bool = True
    jitter_ratio: float = 0.25
    random_uniform: RandomUniform = field(default=random.uniform, repr=False, compare=False)

    def delays(self, max_attempts: int) -> list[float]:
        if max_attempts < 1:
            msg = "max_attempts must be positive"
            raise ValueError(msg)

        delays = []
        for attempt in range(max_attempts - 1):
            delay = self.base_delay * (self.factor**attempt)
            delay = min(delay, self.max_delay)
            if self.jitter:
                delay = self.random_uniform(
                    delay * (1 - self.jitter_ratio),
                    delay * (1 + self.jitter_ratio),
                )
            delays.append(delay)
        return delays

    def metadata(self, max_attempts: int) -> dict[str, object]:
        return {
            "type": "exponential_backoff",
            "base_delay": self.base_delay,
            "factor": self.factor,
            "max_delay": self.max_delay,
            "jitter": self.jitter,
            "jitter_ratio": self.jitter_ratio,
            "max_attempts": max_attempts,
            "max_retries": max_attempts - 1,
            "nominal_delays": ExponentialBackoffPolicy(
                base_delay=self.base_delay,
                factor=self.factor,
                max_delay=self.max_delay,
                jitter=False,
            ).delays(max_attempts),
        }


DEFAULT_RETRY_POLICY = ExponentialBackoffPolicy(jitter=False)
