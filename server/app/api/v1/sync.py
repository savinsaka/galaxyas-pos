from fastapi import APIRouter, HTTPException, status

from app.core.deps import CurrentUserDep, DbSession
from app.schemas.sync import PullResponse, PushRequest, PushResponse
from app.services import sync_service

router = APIRouter(prefix="/sync", tags=["sync"])


@router.post("/push", response_model=PushResponse)
def push(req: PushRequest, current: CurrentUserDep, db: DbSession) -> PushResponse:
    # A client may only push data for its own store.
    if current.store_id and current.store_id != req.store_id:
        raise HTTPException(status.HTTP_403_FORBIDDEN, "Store scope tidak cocok")
    return sync_service.process_push(db, req)


@router.get("/pull", response_model=PullResponse)
def pull(store_id: str, since: int, current: CurrentUserDep, db: DbSession) -> PullResponse:
    if current.store_id and current.store_id != store_id:
        raise HTTPException(status.HTTP_403_FORBIDDEN, "Store scope tidak cocok")
    return sync_service.process_pull(db, store_id, since)
