import dataclasses
from typing import Any, Literal, Optional
from collections.abc import Iterable
from dlt.common.data_types.typing import TDataType


@dataclasses.dataclass
class SourceResponse:
    name: str
    items: Iterable[Any]
    primary_keys: list[str] | None
    column_hints: dict[str, TDataType | None] | None = None  # Legacy support for DLT sources
    partition_count: Optional[int] = None
    partition_size: Optional[int] = None


PartitionMode = Literal["md5", "numerical", "datetime"]
