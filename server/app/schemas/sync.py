from typing import Any

from pydantic import BaseModel, Field


class PushItem(BaseModel):
    queue_id: str
    entity_type: str
    entity_id: str
    operation: str  # 'insert' | 'update' | 'delete'
    payload: dict[str, Any]
    base_updated_at: int | None = None


class PushRequest(BaseModel):
    store_id: str
    changes: list[PushItem] = Field(default_factory=list)


class ConflictOut(BaseModel):
    queue_id: str
    entity_type: str
    entity_id: str
    server_payload: dict[str, Any]
    conflict_field: str | None = None


class PushResponse(BaseModel):
    accepted: list[str] = Field(default_factory=list)
    conflicts: list[ConflictOut] = Field(default_factory=list)


class PullChange(BaseModel):
    entity_type: str
    entity_id: str
    payload: dict[str, Any]
    server_updated_at: int
    deleted: bool = False


class PullResponse(BaseModel):
    changes: list[PullChange] = Field(default_factory=list)
    server_time: int
