from starlette.exceptions import HTTPException

from utils.http import OrJSONResponse


class ExceptionBase(HTTPException):
    status_code: int = 500
    error_code: str = None
    detail: str = None

    def __init__(self, error_code: str, detail: str = None, status_code: int = 500):
        self.status_code = status_code
        self.error_code = error_code
        self.detail = detail

    def json_response(self):
        return OrJSONResponse(
            {"detail": {"error_code": self.error_code, "message": self.detail}},
            status_code=self.status_code,
        )
