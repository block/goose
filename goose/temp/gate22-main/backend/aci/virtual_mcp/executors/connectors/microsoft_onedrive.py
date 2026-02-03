import csv
import io
import json

import httpx
from mcp import types as mcp_types

from aci.common.logging_setup import get_logger
from aci.common.schemas.virtual_mcp import VirtualMCPAuthTokenData
from aci.virtual_mcp.executors.connectors.base import BaseConnector

logger = get_logger(__name__)


class MicrosoftOnedrive(BaseConnector):
    """
    Microsoft OneDrive Connector for text file operations.
    """

    def __init__(
        self,
        auth_token_data: VirtualMCPAuthTokenData,
    ):
        super().__init__(auth_token_data)
        self.access_token = auth_token_data.token
        self.base_url = "https://graph.microsoft.com/v1.0"

    def read_text_file_content(self, item_id: str) -> mcp_types.CallToolResult:
        """
        Read the content of a text file from OneDrive by its item ID.

        Args:
            item_id: The identifier of the driveItem file to read

        Returns:
            mcp_types.CallToolResult: mcp compatible call tool result containing the response
            from the OneDrive API
        """
        logger.debug(f"Reading text file from OneDrive: {item_id}")

        # Construct API URLs
        metadata_url = f"{self.base_url}/me/drive/items/{item_id}"
        content_url = f"{self.base_url}/me/drive/items/{item_id}/content"

        headers = {
            "Authorization": f"Bearer {self.access_token}",
        }

        # Get file metadata first
        metadata_response = httpx.get(metadata_url, headers=headers, timeout=30.0)
        metadata_response.raise_for_status()
        metadata = metadata_response.json()

        # Check if it's a file (not a folder)
        if "file" not in metadata:
            raise Exception(f"Item '{item_id}' is not a file or does not exist")
        if not isinstance(metadata, dict):
            logger.error("metadata is not a dictionary")
            metadata = {}

        # Get file content - this will follow the 302 redirect automatically
        content_response = httpx.get(content_url, headers=headers, timeout=30.0)
        content_response.raise_for_status()

        # Decode content as text
        try:
            content = content_response.text
        except UnicodeDecodeError:
            logger.warning(f"File {item_id} contains non-text content, attempting UTF-8 decode")
            content = content_response.content.decode("utf-8", errors="replace")

        logger.info(f"Successfully read file: {item_id}, size: {len(content)} characters")

        result = {
            "content": content,
            "id": metadata.get("id", ""),
            "name": metadata.get("name", ""),
            "path": metadata.get("parentReference", {}).get("path", "")
            + "/"
            + metadata.get("name", ""),
            "size": metadata.get("size", 0),
            "mime_type": metadata.get("file", {}).get("mimeType", ""),
            "created_datetime": metadata.get("createdDateTime", ""),
            "modified_datetime": metadata.get("lastModifiedDateTime", ""),
        }
        return mcp_types.CallToolResult(
            structuredContent=result,
            content=[mcp_types.TextContent(type="text", text=json.dumps(result))],
        )

    def create_excel_from_csv(
        self, csv_data: str, parent_folder_id: str, filename: str | None = None
    ) -> mcp_types.CallToolResult:
        """
        Convert CSV data to a properly formatted CSV file and save it to OneDrive.
        This creates a CSV file that can be opened in Excel.

        Args:
            csv_data: The CSV data as a string to save
            parent_folder_id: The identifier of the parent folder where the CSV file will be created
            filename: Optional custom name for the CSV file (without .csv extension)

        Returns:
            mcp_types.CallToolResult: mcp compatible call tool result containing the response
            from the OneDrive API
        """
        logger.debug(f"Creating CSV file on OneDrive, folder: {parent_folder_id}")

        # Parse and validate CSV data using built-in csv module
        csv_reader = csv.reader(io.StringIO(csv_data))
        rows = list(csv_reader)

        if not rows:
            raise Exception("CSV data is empty")

        # Determine filename
        if not filename:
            filename = "converted_data"

        # Ensure .csv extension
        if not filename.endswith(".csv"):
            filename += ".csv"

        # Upload CSV file to OneDrive using the existing text file creation method
        upload_url = f"{self.base_url}/me/drive/items/{parent_folder_id}:/{filename}:/content"

        headers = {
            "Authorization": f"Bearer {self.access_token}",
            "Content-Type": "text/csv",
        }

        upload_response = httpx.put(
            upload_url,
            headers=headers,
            content=csv_data.encode("utf-8"),
            timeout=60.0,
        )
        upload_response.raise_for_status()

        result = upload_response.json()
        if not isinstance(result, dict):
            logger.error("result is not a dictionary")
            return mcp_types.CallToolResult(
                isError=True,
                content=[
                    mcp_types.TextContent(
                        type="text", text="Failed to create CSV file from CSV data"
                    )
                ],
            )

        logger.debug(f"Successfully created CSV file: {filename}, ID: {result.get('id', '')}")

        result = {
            "id": result.get("id", ""),
            "name": result.get("name", ""),
            "path": result.get("parentReference", {}).get("path", "")
            + "/"
            + result.get("name", ""),
            "size": result.get("size", 0),
            "mime_type": result.get("file", {}).get("mimeType", ""),
            "created_datetime": result.get("createdDateTime", ""),
            "modified_datetime": result.get("lastModifiedDateTime", ""),
            "download_url": result.get("@microsoft.graph.downloadUrl", ""),
            "rows_converted": len(rows),
            "columns_converted": len(rows[0]) if rows else 0,
            "note": "CSV file created successfully. This file can be opened in Excel.",
        }
        return mcp_types.CallToolResult(
            structuredContent=result,
            content=[mcp_types.TextContent(type="text", text=json.dumps(result))],
        )
