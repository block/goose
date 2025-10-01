import requests
import json
import os
from typing import List, Dict, Optional, Any, Tuple
from datetime import datetime, timedelta
from mcp.server.fastmcp import FastMCP

# Directory to store weather search results
WEATHER_DIR = "weather_data"

# Initialize FastMCP server
mcp = FastMCP("weather-search")

# Base URL for National Weather Service API
NWS_BASE_URL = "https://api.weather.gov"

def get_headers() -> Dict[str, str]:
    """Get headers for NWS API requests with required User-Agent."""
    return {
        "User-Agent": "WeatherSearchMCP/1.0 (weather-search-mcp, support@example.com)",
        "Accept": "application/geo+json"
    }

def make_nws_request(endpoint: str) -> Optional[Dict[str, Any]]:
    """Make a request to the NWS API with proper error handling."""
    try:
        response = requests.get(endpoint, headers=get_headers(), timeout=10)
        response.raise_for_status()
        return response.json()
    except requests.exceptions.RequestException as e:
        print(f"Error making request to {endpoint}: {str(e)}")
        return None

def save_weather_data(data: Dict[str, Any], filename: str) -> str:
    """Save weather data to file and return file path."""
    os.makedirs(WEATHER_DIR, exist_ok=True)
    file_path = os.path.join(WEATHER_DIR, f"{filename}.json")
    
    with open(file_path, "w") as f:
        json.dump(data, f, indent=2)
    
    return file_path

@mcp.tool()
def get_location_info(latitude: float, longitude: float) -> Dict[str, Any]:
    """
    Get location information and grid data for coordinates.
    
    Args:
        latitude: Latitude coordinate (e.g., 39.7456)
        longitude: Longitude coordinate (e.g., -97.0892)
        
    Returns:
        Dict containing location info, grid coordinates, and forecast endpoints
    """
    
    endpoint = f"{NWS_BASE_URL}/points/{latitude},{longitude}"
    data = make_nws_request(endpoint)
    
    if not data:
        return {"error": f"Failed to get location info for {latitude},{longitude}"}
    
    try:
        properties = data.get("properties", {})
        result = {
            "location": {
                "latitude": latitude,
                "longitude": longitude,
                "city": properties.get("relativeLocation", {}).get("properties", {}).get("city", "Unknown"),
                "state": properties.get("relativeLocation", {}).get("properties", {}).get("state", "Unknown")
            },
            "grid": {
                "office": properties.get("cwa"),
                "gridX": properties.get("gridX"),
                "gridY": properties.get("gridY")
            },
            "forecast_endpoints": {
                "forecast": properties.get("forecast"),
                "forecast_hourly": properties.get("forecastHourly"),
                "forecast_grid_data": properties.get("forecastGridData")
            },
            "observation_stations": properties.get("observationStations"),
            "fire_weather_zone": properties.get("fireWeatherZone"),
            "forecast_zone": properties.get("forecastZone"),
            "county": properties.get("county"),
            "time_zone": properties.get("timeZone")
        }
        
        # Save location data
        search_id = f"location_{latitude}_{longitude}_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
        save_weather_data(result, search_id)
        result["search_id"] = search_id
        
        return result
    
    except Exception as e:
        return {"error": f"Error processing location data: {str(e)}"}

@mcp.tool()
def get_current_conditions(latitude: float, longitude: float) -> Dict[str, Any]:
    """
    Get current weather conditions for a location.
    
    Args:
        latitude: Latitude coordinate
        longitude: Longitude coordinate
        
    Returns:
        Dict containing current weather observations
    """
    
    # First get location info to find observation stations
    location_info = get_location_info(latitude, longitude)
    if "error" in location_info:
        return location_info
    
    # Get observation stations
    stations_url = location_info.get("observation_stations")
    if not stations_url:
        return {"error": "No observation stations found for this location"}
    
    stations_data = make_nws_request(stations_url)
    if not stations_data or not stations_data.get("features"):
        return {"error": "Failed to get observation stations"}
    
    # Try to get observations from the first few stations
    for station_feature in stations_data["features"][:3]:
        station_id = station_feature["properties"]["stationIdentifier"]
        obs_endpoint = f"{NWS_BASE_URL}/stations/{station_id}/observations/latest"
        obs_data = make_nws_request(obs_endpoint)
        
        if obs_data and obs_data.get("properties"):
            properties = obs_data["properties"]
            
            result = {
                "location": location_info["location"],
                "station": {
                    "id": station_id,
                    "name": station_feature["properties"].get("name", "Unknown")
                },
                "observation_time": properties.get("timestamp"),
                "conditions": {
                    "temperature": {
                        "value": properties.get("temperature", {}).get("value"),
                        "unit": properties.get("temperature", {}).get("unitCode", "").replace("wmoUnit:", "")
                    },
                    "dewpoint": {
                        "value": properties.get("dewpoint", {}).get("value"),
                        "unit": properties.get("dewpoint", {}).get("unitCode", "").replace("wmoUnit:", "")
                    },
                    "wind_direction": {
                        "value": properties.get("windDirection", {}).get("value"),
                        "unit": properties.get("windDirection", {}).get("unitCode", "").replace("wmoUnit:", "")
                    },
                    "wind_speed": {
                        "value": properties.get("windSpeed", {}).get("value"),
                        "unit": properties.get("windSpeed", {}).get("unitCode", "").replace("wmoUnit:", "")
                    },
                    "wind_gust": {
                        "value": properties.get("windGust", {}).get("value"),
                        "unit": properties.get("windGust", {}).get("unitCode", "").replace("wmoUnit:", "")
                    },
                    "barometric_pressure": {
                        "value": properties.get("barometricPressure", {}).get("value"),
                        "unit": properties.get("barometricPressure", {}).get("unitCode", "").replace("wmoUnit:", "")
                    },
                    "sea_level_pressure": {
                        "value": properties.get("seaLevelPressure", {}).get("value"),
                        "unit": properties.get("seaLevelPressure", {}).get("unitCode", "").replace("wmoUnit:", "")
                    },
                    "visibility": {
                        "value": properties.get("visibility", {}).get("value"),
                        "unit": properties.get("visibility", {}).get("unitCode", "").replace("wmoUnit:", "")
                    },
                    "max_temperature_last_24_hours": {
                        "value": properties.get("maxTemperatureLast24Hours", {}).get("value"),
                        "unit": properties.get("maxTemperatureLast24Hours", {}).get("unitCode", "").replace("wmoUnit:", "")
                    },
                    "min_temperature_last_24_hours": {
                        "value": properties.get("minTemperatureLast24Hours", {}).get("value"),
                        "unit": properties.get("minTemperatureLast24Hours", {}).get("unitCode", "").replace("wmoUnit:", "")
                    },
                    "precipitation_last_hour": {
                        "value": properties.get("precipitationLastHour", {}).get("value"),
                        "unit": properties.get("precipitationLastHour", {}).get("unitCode", "").replace("wmoUnit:", "")
                    },
                    "precipitation_last_3_hours": {
                        "value": properties.get("precipitationLast3Hours", {}).get("value"),
                        "unit": properties.get("precipitationLast3Hours", {}).get("unitCode", "").replace("wmoUnit:", "")
                    },
                    "precipitation_last_6_hours": {
                        "value": properties.get("precipitationLast6Hours", {}).get("value"),
                        "unit": properties.get("precipitationLast6Hours", {}).get("unitCode", "").replace("wmoUnit:", "")
                    },
                    "relative_humidity": {
                        "value": properties.get("relativeHumidity", {}).get("value"),
                        "unit": properties.get("relativeHumidity", {}).get("unitCode", "").replace("wmoUnit:", "")
                    },
                    "wind_chill": {
                        "value": properties.get("windChill", {}).get("value"),
                        "unit": properties.get("windChill", {}).get("unitCode", "").replace("wmoUnit:", "")
                    },
                    "heat_index": {
                        "value": properties.get("heatIndex", {}).get("value"),
                        "unit": properties.get("heatIndex", {}).get("unitCode", "").replace("wmoUnit:", "")
                    },
                    "cloud_layers": properties.get("cloudLayers", []),
                    "present_weather": properties.get("presentWeather", []),
                    "text_description": properties.get("textDescription")
                }
            }
            
            # Save current conditions data
            search_id = f"current_{latitude}_{longitude}_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
            save_weather_data(result, search_id)
            result["search_id"] = search_id
            
            return result
    
    return {"error": "Unable to get current conditions from nearby stations"}

@mcp.tool()
def get_weather_forecast(latitude: float, longitude: float, hourly: bool = False) -> Dict[str, Any]:
    """
    Get weather forecast for a location.
    
    Args:
        latitude: Latitude coordinate
        longitude: Longitude coordinate
        hourly: If True, get hourly forecast; if False, get daily forecast
        
    Returns:
        Dict containing weather forecast
    """
    
    # Get location info and grid coordinates
    location_info = get_location_info(latitude, longitude)
    if "error" in location_info:
        return location_info
    
    grid = location_info["grid"]
    office = grid["office"]
    grid_x = grid["gridX"]
    grid_y = grid["gridY"]
    
    if not all([office, grid_x, grid_y]):
        return {"error": "Invalid grid coordinates for location"}
    
    # Build forecast endpoint
    forecast_type = "forecast/hourly" if hourly else "forecast"
    endpoint = f"{NWS_BASE_URL}/gridpoints/{office}/{grid_x},{grid_y}/{forecast_type}"
    
    forecast_data = make_nws_request(endpoint)
    if not forecast_data:
        return {"error": f"Failed to get forecast from {endpoint}"}
    
    try:
        properties = forecast_data.get("properties", {})
        periods = properties.get("periods", [])
        
        result = {
            "location": location_info["location"],
            "grid": grid,
            "forecast_type": "hourly" if hourly else "daily",
            "updated": properties.get("updated"),
            "generated_at": properties.get("generatedAt"),
            "elevation": properties.get("elevation"),
            "periods": []
        }
        
        for period in periods:
            period_data = {
                "number": period.get("number"),
                "name": period.get("name"),
                "start_time": period.get("startTime"),
                "end_time": period.get("endTime"),
                "is_daytime": period.get("isDaytime"),
                "temperature": period.get("temperature"),
                "temperature_unit": period.get("temperatureUnit"),
                "temperature_trend": period.get("temperatureTrend"),
                "probability_of_precipitation": period.get("probabilityOfPrecipitation", {}).get("value"),
                "dewpoint": period.get("dewpoint", {}).get("value"),
                "relative_humidity": period.get("relativeHumidity", {}).get("value"),
                "wind_speed": period.get("windSpeed"),
                "wind_direction": period.get("windDirection"),
                "icon": period.get("icon"),
                "short_forecast": period.get("shortForecast"),
                "detailed_forecast": period.get("detailedForecast")
            }
            result["periods"].append(period_data)
        
        # Save forecast data
        forecast_id = f"forecast_{'hourly' if hourly else 'daily'}_{latitude}_{longitude}_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
        save_weather_data(result, forecast_id)
        result["search_id"] = forecast_id
        
        return result
    
    except Exception as e:
        return {"error": f"Error processing forecast data: {str(e)}"}

@mcp.tool()
def get_weather_alerts(
    area: Optional[str] = None,
    region: Optional[str] = None,
    zone: Optional[str] = None,
    point: Optional[Tuple[float, float]] = None,
    active_only: bool = True,
    urgency: Optional[str] = None,
    severity: Optional[str] = None,
    certainty: Optional[str] = None
) -> Dict[str, Any]:
    """
    Get weather alerts for a specified area or location.
    
    Args:
        area: Two-letter state/territory code (e.g., 'KS', 'TX')
        region: Region code (e.g., 'US')
        zone: Zone ID (e.g., 'ILZ014')
        point: Tuple of (latitude, longitude) for point-based alerts
        active_only: If True, get only active alerts
        urgency: Filter by urgency (Immediate, Expected, Future, Past, Unknown)
        severity: Filter by severity (Extreme, Severe, Moderate, Minor, Unknown)
        certainty: Filter by certainty (Observed, Likely, Possible, Unlikely, Unknown)
        
    Returns:
        Dict containing weather alerts
    """
    
    # Build alerts endpoint
    if active_only:
        endpoint = f"{NWS_BASE_URL}/alerts/active"
    else:
        endpoint = f"{NWS_BASE_URL}/alerts"
    
    # Build query parameters
    params = []
    if area:
        params.append(f"area={area}")
    if region:
        params.append(f"region={region}")
    if zone:
        params.append(f"zone={zone}")
    if point:
        lat, lon = point
        params.append(f"point={lat},{lon}")
    if urgency:
        params.append(f"urgency={urgency}")
    if severity:
        params.append(f"severity={severity}")
    if certainty:
        params.append(f"certainty={certainty}")
    
    if params:
        endpoint += "?" + "&".join(params)
    
    alerts_data = make_nws_request(endpoint)
    if not alerts_data:
        return {"error": f"Failed to get alerts from {endpoint}"}
    
    try:
        features = alerts_data.get("features", [])
        
        result = {
            "search_parameters": {
                "area": area,
                "region": region,
                "zone": zone,
                "point": point,
                "active_only": active_only,
                "urgency": urgency,
                "severity": severity,
                "certainty": certainty
            },
            "total_alerts": len(features),
            "alerts": []
        }
        
        for feature in features:
            properties = feature.get("properties", {})
            alert_data = {
                "id": properties.get("id"),
                "area_desc": properties.get("areaDesc"),
                "geocode": properties.get("geocode"),
                "sent": properties.get("sent"),
                "effective": properties.get("effective"),
                "onset": properties.get("onset"),
                "expires": properties.get("expires"),
                "ends": properties.get("ends"),
                "status": properties.get("status"),
                "message_type": properties.get("messageType"),
                "category": properties.get("category"),
                "severity": properties.get("severity"),
                "certainty": properties.get("certainty"),
                "urgency": properties.get("urgency"),
                "response": properties.get("response"),
                "sender_code": properties.get("senderCode"),
                "sender_name": properties.get("senderName"),
                "headline": properties.get("headline"),
                "description": properties.get("description"),
                "instruction": properties.get("instruction"),
                "event": properties.get("event"),
                "parameters": properties.get("parameters", {}),
                "geometry": feature.get("geometry")
            }
            result["alerts"].append(alert_data)
        
        # Save alerts data
        alert_id = f"alerts_{area or 'all'}_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
        save_weather_data(result, alert_id)
        result["search_id"] = alert_id
        
        return result
    
    except Exception as e:
        return {"error": f"Error processing alerts data: {str(e)}"}

@mcp.tool()
def get_weather_data_details(search_id: str) -> str:
    """
    Get detailed information about a specific weather search.
    
    Args:
        search_id: The search ID returned from weather tools
        
    Returns:
        JSON string with detailed weather information
    """
    
    file_path = os.path.join(WEATHER_DIR, f"{search_id}.json")
    
    if not os.path.exists(file_path):
        return f"No weather data found with ID: {search_id}"
    
    try:
        with open(file_path, "r") as f:
            weather_data = json.load(f)
        return json.dumps(weather_data, indent=2)
    except (FileNotFoundError, json.JSONDecodeError) as e:
        return f"Error reading weather data for {search_id}: {str(e)}"

@mcp.tool()
def filter_forecast_by_conditions(
    search_id: str,
    min_temp: Optional[float] = None,
    max_temp: Optional[float] = None,
    max_precipitation_chance: Optional[int] = None,
    wind_speed_threshold: Optional[str] = None
) -> str:
    """
    Filter forecast data by weather conditions.
    
    Args:
        search_id: The search ID returned from get_weather_forecast
        min_temp: Minimum temperature filter
        max_temp: Maximum temperature filter
        max_precipitation_chance: Maximum precipitation probability (0-100)
        wind_speed_threshold: Maximum wind speed (e.g., "15 mph")
        
    Returns:
        JSON string with filtered forecast results
    """
    
    file_path = os.path.join(WEATHER_DIR, f"{search_id}.json")
    
    if not os.path.exists(file_path):
        return f"No weather data found with ID: {search_id}"
    
    try:
        with open(file_path, "r") as f:
            weather_data = json.load(f)
        
        if weather_data.get("forecast_type") is None:
            return f"Data with ID {search_id} is not forecast data"
        
        def condition_filter(period):
            # Temperature filters
            temp = period.get("temperature")
            if temp is not None:
                if min_temp is not None and temp < min_temp:
                    return False
                if max_temp is not None and temp > max_temp:
                    return False
            
            # Precipitation filter
            precip_chance = period.get("probability_of_precipitation")
            if precip_chance is not None and max_precipitation_chance is not None:
                if precip_chance > max_precipitation_chance:
                    return False
            
            # Wind speed filter (simplified - would need more parsing for real use)
            if wind_speed_threshold is not None:
                wind_speed_str = period.get("wind_speed", "")
                # This is a simplified check - in practice, you'd want to parse the wind speed string
                if wind_speed_threshold.lower() in wind_speed_str.lower():
                    return False
            
            return True
        
        filtered_periods = [p for p in weather_data.get("periods", []) if condition_filter(p)]
        
        result = {
            "search_id": search_id,
            "filters_applied": {
                "min_temp": min_temp,
                "max_temp": max_temp,
                "max_precipitation_chance": max_precipitation_chance,
                "wind_speed_threshold": wind_speed_threshold
            },
            "original_periods": len(weather_data.get("periods", [])),
            "filtered_periods": len(filtered_periods),
            "periods": filtered_periods
        }
        
        return json.dumps(result, indent=2)
        
    except (FileNotFoundError, json.JSONDecodeError) as e:
        return f"Error processing weather data for {search_id}: {str(e)}"

@mcp.resource("weather://searches")
def get_weather_searches() -> str:
    """
    List all available weather searches.
    
    This resource provides a list of all saved weather searches.
    """
    searches = []
    
    if os.path.exists(WEATHER_DIR):
        for filename in os.listdir(WEATHER_DIR):
            if filename.endswith('.json'):
                search_id = filename[:-5]  # Remove .json extension
                file_path = os.path.join(WEATHER_DIR, filename)
                try:
                    with open(file_path, 'r') as f:
                        data = json.load(f)
                        
                        search_type = "Unknown"
                        if "forecast_type" in data:
                            search_type = f"Forecast ({data['forecast_type']})"
                        elif "conditions" in data:
                            search_type = "Current Conditions"
                        elif "alerts" in data:
                            search_type = f"Alerts ({data['total_alerts']} alerts)"
                        elif "grid" in data:
                            search_type = "Location Info"
                        
                        location = data.get("location", {})
                        city = location.get("city", "Unknown")
                        state = location.get("state", "Unknown")
                        lat = location.get("latitude", "N/A")
                        lon = location.get("longitude", "N/A")
                        
                        searches.append({
                            'search_id': search_id,
                            'type': search_type,
                            'location': f"{city}, {state}",
                            'coordinates': f"{lat}, {lon}",
                            'search_time': filename.split('_')[-2] + '_' + filename.split('_')[-1].replace('.json', '') if '_' in filename else 'Unknown'
                        })
                except (json.JSONDecodeError, KeyError):
                    continue
    
    content = "# Weather Searches\n\n"
    if searches:
        content += f"Total searches: {len(searches)}\n\n"
        for search in searches:
            content += f"## {search['search_id']}\n"
            content += f"- **Type**: {search['type']}\n"
            content += f"- **Location**: {search['location']}\n"
            content += f"- **Coordinates**: {search['coordinates']}\n"
            content += f"- **Search Time**: {search['search_time']}\n\n"
            content += "---\n\n"
    else:
        content += "No weather searches found.\n\n"
        content += "Use the weather tools to search for weather information.\n"
    
    return content

@mcp.resource("weather://{search_id}")
def get_weather_search_details(search_id: str) -> str:
    """
    Get detailed information about a specific weather search.
    
    Args:
        search_id: The weather search ID to retrieve details for
    """
    file_path = os.path.join(WEATHER_DIR, f"{search_id}.json")
    
    if not os.path.exists(file_path):
        return f"# Weather Search Not Found: {search_id}\n\nNo weather search found with this ID."
    
    try:
        with open(file_path, 'r') as f:
            weather_data = json.load(f)
        
        location = weather_data.get('location', {})
        content = f"# Weather Search: {search_id}\n\n"
        
        # Location details
        content += f"## Location\n"
        content += f"- **City**: {location.get('city', 'Unknown')}\n"
        content += f"- **State**: {location.get('state', 'Unknown')}\n"
        content += f"- **Latitude**: {location.get('latitude', 'N/A')}\n"
        content += f"- **Longitude**: {location.get('longitude', 'N/A')}\n\n"
        
        # Handle different data types
        if 'forecast_type' in weather_data:
            # Forecast data
            content += f"## Forecast ({weather_data['forecast_type'].title()})\n"
            content += f"- **Updated**: {weather_data.get('updated', 'N/A')}\n"
            content += f"- **Generated**: {weather_data.get('generated_at', 'N/A')}\n"
            content += f"- **Total Periods**: {len(weather_data.get('periods', []))}\n\n"
            
            periods = weather_data.get('periods', [])[:5]  # Show first 5 periods
            if periods:
                content += f"### Sample Periods\n\n"
                for period in periods:
                    content += f"**{period.get('name', 'Unknown')}**\n"
                    content += f"- Temperature: {period.get('temperature', 'N/A')}°{period.get('temperature_unit', 'F')}\n"
                    content += f"- Wind: {period.get('wind_speed', 'N/A')} {period.get('wind_direction', '')}\n"
                    content += f"- Precipitation Chance: {period.get('probability_of_precipitation', 0)}%\n"
                    content += f"- Conditions: {period.get('short_forecast', 'N/A')}\n\n"
        
        elif 'conditions' in weather_data:
            # Current conditions data
            conditions = weather_data['conditions']
            content += f"## Current Conditions\n"
            content += f"- **Observation Time**: {weather_data.get('observation_time', 'N/A')}\n"
            content += f"- **Station**: {weather_data.get('station', {}).get('name', 'Unknown')}\n\n"
            
            temp = conditions.get('temperature', {})
            if temp.get('value') is not None:
                content += f"- **Temperature**: {temp['value']}° {temp.get('unit', 'C')}\n"
            
            wind_speed = conditions.get('wind_speed', {})
            wind_dir = conditions.get('wind_direction', {})
            if wind_speed.get('value') is not None:
                content += f"- **Wind**: {wind_speed['value']} {wind_speed.get('unit', 'km/h')} from {wind_dir.get('value', 'N/A')}°\n"
            
            humidity = conditions.get('relative_humidity', {})
            if humidity.get('value') is not None:
                content += f"- **Humidity**: {humidity['value']}%\n"
            
            pressure = conditions.get('barometric_pressure', {})
            if pressure.get('value') is not None:
                content += f"- **Pressure**: {pressure['value']} {pressure.get('unit', 'Pa')}\n"
            
            if conditions.get('text_description'):
                content += f"- **Description**: {conditions['text_description']}\n"
            
            content += "\n"
        
        elif 'alerts' in weather_data:
            # Alerts data
            content += f"## Weather Alerts\n"
            content += f"- **Total Alerts**: {weather_data.get('total_alerts', 0)}\n\n"
            
            alerts = weather_data.get('alerts', [])[:3]  # Show first 3 alerts
            if alerts:
                content += f"### Active Alerts\n\n"
                for alert in alerts:
                    content += f"**{alert.get('event', 'Unknown Event')}**\n"
                    content += f"- **Severity**: {alert.get('severity', 'Unknown')}\n"
                    content += f"- **Urgency**: {alert.get('urgency', 'Unknown')}\n"
                    content += f"- **Areas**: {alert.get('area_desc', 'N/A')}\n"
                    content += f"- **Effective**: {alert.get('effective', 'N/A')}\n"
                    content += f"- **Expires**: {alert.get('expires', 'N/A')}\n"
                    if alert.get('headline'):
                        content += f"- **Headline**: {alert['headline']}\n"
                    content += "\n"
        
        elif 'grid' in weather_data:
            # Location info data
            grid = weather_data['grid']
            content += f"## Grid Information\n"
            content += f"- **Office**: {grid.get('office', 'N/A')}\n"
            content += f"- **Grid X**: {grid.get('gridX', 'N/A')}\n"
            content += f"- **Grid Y**: {grid.get('gridY', 'N/A')}\n"
            content += f"- **Time Zone**: {weather_data.get('time_zone', 'N/A')}\n"
            content += f"- **Fire Weather Zone**: {weather_data.get('fire_weather_zone', 'N/A')}\n"
            content += f"- **Forecast Zone**: {weather_data.get('forecast_zone', 'N/A')}\n\n"
        
        return content
        
    except json.JSONDecodeError:
        return f"# Error\n\nCorrupted weather data for search ID: {search_id}"

@mcp.prompt()
def weather_planning_prompt(
    location: str,
    start_date: str,
    end_date: str = "",
    activity_type: str = "",
    preferences: str = ""
) -> str:
    """Generate a comprehensive weather planning prompt for Claude."""
    
    prompt = f"""Plan weather-related activities for {location} starting on {start_date}"""
    
    if end_date:
        prompt += f" through {end_date}"
    
    if activity_type:
        prompt += f" for {activity_type} activities"
    
    prompt += "."
    
    if preferences:
        prompt += f" Weather preferences: {preferences}."
    
    prompt += f"""

Please help with the following weather planning tasks:

1. **Location Analysis**: First, determine the coordinates for {location} and use get_location_info() to get grid information and nearby weather stations.

2. **Current Conditions**: Use get_current_conditions() to get the current weather situation for {location}.

3. **Weather Forecast**: Get both daily and hourly forecasts using get_weather_forecast():
   - Daily forecast for overall planning
   - Hourly forecast for detailed timing"""
    
    if activity_type:
        prompt += f"""
   - Focus on weather conditions relevant to {activity_type}"""
    
    prompt += f"""

4. **Weather Alerts**: Check for any active weather alerts using get_weather_alerts() that might affect plans.

5. **Analysis and Recommendations**: Provide:
   - Best times/dates for outdoor activities based on forecast
   - Weather-related precautions or preparations needed
   - Alternative indoor activity suggestions if weather is unfavorable
   - Clothing and equipment recommendations based on conditions"""
    
    if activity_type:
        prompt += f"""
   - Specific considerations for {activity_type} activities"""
    
    prompt += f"""

6. **Detailed Day-by-Day Breakdown**: For each day in the planning period:
   - Morning, afternoon, and evening weather conditions
   - Temperature ranges and feels-like temperatures
   - Precipitation chances and types
   - Wind conditions and visibility
   - UV index and sun/cloud coverage
   - Recommended activity windows

Present the information in a clear, organized format with specific actionable recommendations for weather-dependent planning."""

    return prompt

@mcp.prompt()
def severe_weather_alert_prompt(
    area: str,
    alert_types: List[str] = None
) -> str:
    """Generate a prompt for comprehensive severe weather alert monitoring."""
    
    alert_types_str = ", ".join(alert_types) if alert_types else "all types"
    
    return f"""Monitor and analyze severe weather alerts for {area} focusing on {alert_types_str} of weather hazards.

Please provide comprehensive severe weather monitoring including:

1. **Current Alert Status**: Use get_weather_alerts(area='{area}', active_only=True) to get all active alerts

2. **Alert Analysis**: For each active alert, provide:
   - Severity level and urgency assessment
   - Geographic areas affected
   - Timing (when it starts, peaks, and ends)
   - Specific hazards and impacts expected
   - Recommended actions and preparations

3. **Alert Prioritization**: Rank alerts by:
   - Immediate threat level
   - Potential impact severity
   - Geographic scope
   - Population affected

4. **Safety Recommendations**: Based on active alerts:
   - Immediate actions to take now
   - Preparations for anticipated conditions
   - When to seek shelter or evacuate
   - Communication and emergency contact protocols
   - Supply and resource recommendations

5. **Monitoring Strategy**: Suggest:
   - How often to check for alert updates
   - Key weather indicators to watch
   - Additional information sources
   - When to escalate or implement emergency plans

6. **Recovery Planning**: For post-event:
   - When conditions are expected to improve
   - Damage assessment considerations
   - Recovery resource contacts
   - Follow-up weather monitoring needs

Format the response with clear action items, priority levels, and time-sensitive information prominently displayed. Use the alert filtering tools to focus on the most relevant threats."""

if __name__ == "__main__":
    # Initialize and run the server
    mcp.run(transport='stdio')