from __future__ import annotations

from collections.abc import Mapping


def flatten_content_object(
    value: Mapping[str, object],
) -> dict[str, object]:
    path_flattened: dict[str, object] = {}
    _flatten_mapping(value, prefix="", output=path_flattened)
    return {_shortest_leaf_name(key): item for key, item in path_flattened.items()}


def _flatten_mapping(
    value: Mapping[str, object],
    *,
    prefix: str,
    output: dict[str, object],
) -> None:
    for key, item in value.items():
        path = _join_path(prefix, str(key))
        _flatten_value(item, path=path, output=output)


def _flatten_value(value: object, *, path: str, output: dict[str, object]) -> None:
    if isinstance(value, Mapping):
        _flatten_mapping(value, prefix=path, output=output)
        return

    if isinstance(value, list):
        _flatten_list(value, path=path, output=output)
        return

    output[path] = value


def _flatten_list(values: list[object], *, path: str, output: dict[str, object]) -> None:
    list_path = f"{path}[]"
    if not values:
        output[list_path] = []
        return

    if all(isinstance(item, Mapping) for item in values):
        flattened_items = [
            _flatten_content_object_by_path(item) for item in values if isinstance(item, Mapping)
        ]
        item_keys = sorted({key for item in flattened_items for key in item})
        for item_key in item_keys:
            output[f"{list_path}.{item_key}"] = [item.get(item_key) for item in flattened_items]
        return

    output[list_path] = values


def _join_path(prefix: str, key: str) -> str:
    if not prefix:
        return key
    return f"{prefix}.{key}"


def _flatten_content_object_by_path(value: Mapping[str, object]) -> dict[str, object]:
    flattened: dict[str, object] = {}
    _flatten_mapping(value, prefix="", output=flattened)
    return flattened


def _shortest_leaf_name(path: str) -> str:
    leaf = path.rsplit(".", maxsplit=1)[-1]
    return leaf.removesuffix("[]")
