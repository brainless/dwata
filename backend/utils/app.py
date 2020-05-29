from starlette.requests import Request
from starlette.responses import Response
from starlette.middleware.base import BaseHTTPMiddleware, RequestResponseEndpoint


class DwataAppMiddleware(BaseHTTPMiddleware):
    async def dispatch(
        self, request: Request, call_next: RequestResponseEndpoint
    ) -> Response:
        if "api/item/0/admin_meta_note" in request.url.path:
            request.app.state.IS_DWATA_APP = True
            request.app.state.DWATA_APP_NAME = "note"
        return await call_next(request)