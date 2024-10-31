from datetime import datetime


def formatted_time(time: datetime) -> str:
    return time.astimezone().isoformat(timespec="seconds")
