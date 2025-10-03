# ToolRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | **str** |  | 
**tool_call** | **object** |  | 

## Example

```python
from goose_api.models.tool_request import ToolRequest

# TODO update the JSON string below
json = "{}"
# create an instance of ToolRequest from a JSON string
tool_request_instance = ToolRequest.from_json(json)
# print the JSON string representation of the object
print(ToolRequest.to_json())

# convert the object into a dict
tool_request_dict = tool_request_instance.to_dict()
# create an instance of ToolRequest from a dict
tool_request_from_dict = ToolRequest.from_dict(tool_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


