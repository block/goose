from datetime import datetime, timedelta
from typing import Any, Dict, List
from zoneinfo import ZoneInfo

from googleapiclient.discovery import build
from googleapiclient.errors import HttpError


class GoogleCalendarClient:
    def __init__(self, credentials: dict) -> None:
        self.creds = credentials
        self.service = build("calendar", "v3", credentials=credentials)

    def list_calendars(self) -> List[Dict[str, Any]]:
        try:
            calendars_result = self.service.calendarList().list().execute()
            calendars = calendars_result.get("items", [])
            return calendars
        except HttpError as error:
            print(f"An error occurred: {error}")
            return []

    def list_events_for_today(self) -> List[Dict[str, Any]]:
        try:
            # Get the start and end of the current day in UTC
            now = datetime.now(ZoneInfo("UTC"))
            start_of_day = now.replace(hour=0, minute=0, second=0, microsecond=0)
            end_of_day = start_of_day + timedelta(days=1)

            # Convert to RFC3339 format
            time_min = start_of_day.isoformat()
            time_max = end_of_day.isoformat()

            # Call the Calendar API
            events_result = (
                self.service.events()
                .list(calendarId="primary", timeMin=time_min, timeMax=time_max, singleEvents=True, orderBy="startTime")
                .execute()
            )
            events = events_result.get("items", [])

            if not events:
                print("No events found for today.")
                return []

            return events

        except HttpError as error:
            print(f"An error occurred: {error}")
            return []
