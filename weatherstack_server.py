import requests
import json
import os
from typing import List, Dict, Optional, Any
from datetime import datetime, timedelta
from mcp.server.fastmcp import FastMCP

# Directory to store weather search results
WEATHER_DIR = "weather_data"

# Initialize FastMCP server
mcp = FastMCP("weather-assistant")

def get_weatherstack_key() -> str:
    """Get Weatherstack API key from environment variable."""
    api_key = os.getenv("WEATHERSTACK_API_KEY")
    if not api_key:
        raise ValueError("WEATHERSTACK_API_KEY environment variable is required")
    return api_key

@mcp.tool()
def get_current_weather(
    location: str,
    units: str = "m",
    language: str = "en"
) -> Dict[str, Any]:
    """
    Get current weather information for a specific location.
    
    Args:
        location: Location name, coordinates (lat,lon), IP address, or ZIP code
        units: Temperature unit - 'm' for Celsius, 'f' for Fahrenheit, 's' for Kelvin
        language: Language code (e.g., 'en', 'es', 'fr', 'de')
        
    Returns:
        Dict containing current weather data and metadata
    """
    
    try:
        api_key = get_weatherstack_key()
        
        # Build request parameters
        params = {
            "access_key": api_key,
            "query": location,
            "units": units,
            "language": language
        }
        
        # Make API request
        response = requests.get("https://api.weatherstack.com/current", params=params)
        response.raise_for_status()
        
        weather_data = response.json()
        
        # Check for API errors
        if not weather_data.get("success", True):
            error_info = weather_data.get("error", {})
            return {
                "error": f"API Error {error_info.get('code', 'Unknown')}: {error_info.get('info', 'Unknown error')}"
            }
        
        # Create search identifier
        search_id = f"current_{location.replace(' ', '_')}_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
        
        # Create directory structure
        os.makedirs(WEATHER_DIR, exist_ok=True)
        
        # Process and store weather data
        processed_data = {
            "search_metadata": {
                "search_id": search_id,
                "location_query": location,
                "units": units,
                "language": language,
                "search_type": "current",
                "search_timestamp": datetime.now().isoformat()
            },
            "request": weather_data.get("request", {}),
            "location": weather_data.get("location", {}),
            "current": weather_data.get("current", {})
        }
        
        # Save results to file
        file_path = os.path.join(WEATHER_DIR, f"{search_id}.json")
        with open(file_path, "w") as f:
            json.dump(processed_data, f, indent=2)
        
        print(f"Current weather data saved to: {file_path}")
        
        # Return summary for the user
        current = weather_data.get("current", {})
        location_info = weather_data.get("location", {})
        
        summary = {
            "search_id": search_id,
            "location": {
                "name": location_info.get("name", "Unknown"),
                "country": location_info.get("country", "Unknown"),
                "region": location_info.get("region", "Unknown"),
                "coordinates": f"{location_info.get('lat', 'N/A')}, {location_info.get('lon', 'N/A')}",
                "local_time": location_info.get("localtime", "N/A"),
                "timezone": location_info.get("timezone_id", "N/A")
            },
            "current_weather": {
                "temperature": f"{current.get('temperature', 'N/A')}°{'C' if units == 'm' else 'F' if units == 'f' else 'K'}",
                "description": current.get("weather_descriptions", ["N/A"])[0],
                "feels_like": f"{current.get('feelslike', 'N/A')}°{'C' if units == 'm' else 'F' if units == 'f' else 'K'}",
                "humidity": f"{current.get('humidity', 'N/A')}%",
                "wind": f"{current.get('wind_speed', 'N/A')} {'km/h' if units in ['m', 's'] else 'mph'} {current.get('wind_dir', '')}",
                "pressure": f"{current.get('pressure', 'N/A')} mb",
                "visibility": f"{current.get('visibility', 'N/A')} {'km' if units in ['m', 's'] else 'miles'}",
                "uv_index": current.get('uv_index', 'N/A'),
                "cloud_cover": f"{current.get('cloudcover', 'N/A')}%"
            },
            "air_quality": current.get("air_quality", {}),
            "search_parameters": processed_data["search_metadata"]
        }
        
        return summary
        
    except requests.exceptions.RequestException as e:
        return {"error": f"API request failed: {str(e)}"}
    except ValueError as e:
        return {"error": str(e)}
    except Exception as e:
        return {"error": f"Unexpected error: {str(e)}"}

@mcp.tool()
def get_weather_forecast(
    location: str,
    forecast_days: int = 5,
    hourly: bool = False,
    units: str = "m",
    language: str = "en"
) -> Dict[str, Any]:
    """
    Get weather forecast for a specific location (Professional Plan required).
    
    Args:
        location: Location name, coordinates (lat,lon), IP address, or ZIP code
        forecast_days: Number of forecast days (1-14)
        hourly: Include hourly forecast data
        units: Temperature unit - 'm' for Celsius, 'f' for Fahrenheit, 's' for Kelvin
        language: Language code (e.g., 'en', 'es', 'fr', 'de')
        
    Returns:
        Dict containing weather forecast data and metadata
    """
    
    try:
        api_key = get_weatherstack_key()
        
        # Validate forecast_days
        if not 1 <= forecast_days <= 14:
            return {"error": "forecast_days must be between 1 and 14"}
        
        # Build request parameters
        params = {
            "access_key": api_key,
            "query": location,
            "forecast_days": forecast_days,
            "hourly": 1 if hourly else 0,
            "units": units,
            "language": language
        }
        
        # Make API request
        response = requests.get("https://api.weatherstack.com/forecast", params=params)
        response.raise_for_status()
        
        weather_data = response.json()
        
        # Check for API errors
        if not weather_data.get("success", True):
            error_info = weather_data.get("error", {})
            return {
                "error": f"API Error {error_info.get('code', 'Unknown')}: {error_info.get('info', 'Unknown error')}"
            }
        
        # Create search identifier
        search_id = f"forecast_{location.replace(' ', '_')}_{forecast_days}d_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
        
        # Create directory structure
        os.makedirs(WEATHER_DIR, exist_ok=True)
        
        # Process and store weather data
        processed_data = {
            "search_metadata": {
                "search_id": search_id,
                "location_query": location,
                "forecast_days": forecast_days,
                "hourly": hourly,
                "units": units,
                "language": language,
                "search_type": "forecast",
                "search_timestamp": datetime.now().isoformat()
            },
            "request": weather_data.get("request", {}),
            "location": weather_data.get("location", {}),
            "current": weather_data.get("current", {}),
            "forecast": weather_data.get("forecast", {})
        }
        
        # Save results to file
        file_path = os.path.join(WEATHER_DIR, f"{search_id}.json")
        with open(file_path, "w") as f:
            json.dump(processed_data, f, indent=2)
        
        print(f"Weather forecast data saved to: {file_path}")
        
        # Return summary for the user
        location_info = weather_data.get("location", {})
        forecast_data = weather_data.get("forecast", {})
        
        summary = {
            "search_id": search_id,
            "location": {
                "name": location_info.get("name", "Unknown"),
                "country": location_info.get("country", "Unknown"),
                "region": location_info.get("region", "Unknown"),
                "coordinates": f"{location_info.get('lat', 'N/A')}, {location_info.get('lon', 'N/A')}",
                "local_time": location_info.get("localtime", "N/A"),
                "timezone": location_info.get("timezone_id", "N/A")
            },
            "forecast_summary": {
                "total_days": len(forecast_data),
                "date_range": f"{min(forecast_data.keys()) if forecast_data else 'N/A'} to {max(forecast_data.keys()) if forecast_data else 'N/A'}",
                "includes_hourly": hourly
            },
            "search_parameters": processed_data["search_metadata"]
        }
        
        return summary
        
    except requests.exceptions.RequestException as e:
        return {"error": f"API request failed: {str(e)}"}
    except ValueError as e:
        return {"error": str(e)}
    except Exception as e:
        return {"error": f"Unexpected error: {str(e)}"}

@mcp.tool()
def get_historical_weather(
    location: str,
    date: str,
    end_date: Optional[str] = None,
    hourly: bool = False,
    units: str = "m",
    language: str = "en"
) -> Dict[str, Any]:
    """
    Get historical weather data for a specific location and date(s) (Standard Plan required).
    
    Args:
        location: Location name, coordinates (lat,lon), IP address, or ZIP code
        date: Historical date in YYYY-MM-DD format (back to 2015)
        end_date: End date for date range queries (optional)
        hourly: Include hourly historical data
        units: Temperature unit - 'm' for Celsius, 'f' for Fahrenheit, 's' for Kelvin
        language: Language code (e.g., 'en', 'es', 'fr', 'de')
        
    Returns:
        Dict containing historical weather data and metadata
    """
    
    try:
        api_key = get_weatherstack_key()
        
        # Build request parameters
        params = {
            "access_key": api_key,
            "query": location,
            "hourly": 1 if hourly else 0,
            "units": units,
            "language": language
        }
        
        # Add date parameters
        if end_date:
            params["historical_date_start"] = date
            params["historical_date_end"] = end_date
        else:
            params["historical_date"] = date
        
        # Make API request
        response = requests.get("https://api.weatherstack.com/historical", params=params)
        response.raise_for_status()
        
        weather_data = response.json()
        
        # Check for API errors
        if not weather_data.get("success", True):
            error_info = weather_data.get("error", {})
            return {
                "error": f"API Error {error_info.get('code', 'Unknown')}: {error_info.get('info', 'Unknown error')}"
            }
        
        # Create search identifier
        date_range = f"{date}_to_{end_date}" if end_date else date
        search_id = f"historical_{location.replace(' ', '_')}_{date_range}_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
        
        # Create directory structure
        os.makedirs(WEATHER_DIR, exist_ok=True)
        
        # Process and store weather data
        processed_data = {
            "search_metadata": {
                "search_id": search_id,
                "location_query": location,
                "start_date": date,
                "end_date": end_date,
                "hourly": hourly,
                "units": units,
                "language": language,
                "search_type": "historical",
                "search_timestamp": datetime.now().isoformat()
            },
            "request": weather_data.get("request", {}),
            "location": weather_data.get("location", {}),
            "current": weather_data.get("current", {}),
            "historical": weather_data.get("historical", {})
        }
        
        # Save results to file
        file_path = os.path.join(WEATHER_DIR, f"{search_id}.json")
        with open(file_path, "w") as f:
            json.dump(processed_data, f, indent=2)
        
        print(f"Historical weather data saved to: {file_path}")
        
        # Return summary for the user
        location_info = weather_data.get("location", {})
        historical_data = weather_data.get("historical", {})
        
        summary = {
            "search_id": search_id,
            "location": {
                "name": location_info.get("name", "Unknown"),
                "country": location_info.get("country", "Unknown"),
                "region": location_info.get("region", "Unknown"),
                "coordinates": f"{location_info.get('lat', 'N/A')}, {location_info.get('lon', 'N/A')}",
                "timezone": location_info.get("timezone_id", "N/A")
            },
            "historical_summary": {
                "total_days": len(historical_data),
                "date_range": f"{date} to {end_date}" if end_date else date,
                "includes_hourly": hourly
            },
            "search_parameters": processed_data["search_metadata"]
        }
        
        return summary
        
    except requests.exceptions.RequestException as e:
        return {"error": f"API request failed: {str(e)}"}
    except ValueError as e:
        return {"error": str(e)}
    except Exception as e:
        return {"error": f"Unexpected error: {str(e)}"}

@mcp.tool()
def search_locations(query: str) -> Dict[str, Any]:
    """
    Search for locations using the autocomplete endpoint (Standard Plan required).
    
    Args:
        query: Location search query (city name, partial name, etc.)
        
    Returns:
        Dict containing location search results
    """
    
    try:
        api_key = get_weatherstack_key()
        
        # Build request parameters
        params = {
            "access_key": api_key,
            "query": query
        }
        
        # Make API request
        response = requests.get("https://api.weatherstack.com/autocomplete", params=params)
        response.raise_for_status()
        
        location_data = response.json()
        
        # Check for API errors
        if not location_data.get("success", True):
            error_info = location_data.get("error", {})
            return {
                "error": f"API Error {error_info.get('code', 'Unknown')}: {error_info.get('info', 'Unknown error')}"
            }
        
        return {
            "query": query,
            "results_count": location_data.get("request", {}).get("results", 0),
            "locations": location_data.get("results", [])
        }
        
    except requests.exceptions.RequestException as e:
        return {"error": f"API request failed: {str(e)}"}
    except ValueError as e:
        return {"error": str(e)}
    except Exception as e:
        return {"error": f"Unexpected error: {str(e)}"}

@mcp.tool()
def get_weather_details(search_id: str) -> str:
    """
    Get detailed information about a specific weather search.
    
    Args:
        search_id: The search ID returned from weather tools
        
    Returns:
        JSON string with detailed weather information
    """
    
    file_path = os.path.join(WEATHER_DIR, f"{search_id}.json")
    
    if not os.path.exists(file_path):
        return f"No weather search found with ID: {search_id}"
    
    try:
        with open(file_path, "r") as f:
            weather_data = json.load(f)
        return json.dumps(weather_data, indent=2)
    except (FileNotFoundError, json.JSONDecodeError) as e:
        return f"Error reading weather data for {search_id}: {str(e)}"

@mcp.tool()
def compare_weather(
    locations: List[str],
    units: str = "m",
    language: str = "en"
) -> Dict[str, Any]:
    """
    Compare current weather across multiple locations.
    
    Args:
        locations: List of location names to compare
        units: Temperature unit - 'm' for Celsius, 'f' for Fahrenheit, 's' for Kelvin
        language: Language code (e.g., 'en', 'es', 'fr', 'de')
        
    Returns:
        Dict containing weather comparison data
    """
    
    if len(locations) < 2:
        return {"error": "At least 2 locations required for comparison"}
    
    if len(locations) > 10:
        return {"error": "Maximum 10 locations allowed for comparison"}
    
    comparison_data = {
        "comparison_timestamp": datetime.now().isoformat(),
        "units": units,
        "language": language,
        "locations": [],
        "summary": {
            "hottest": {"location": "", "temperature": float('-inf')},
            "coldest": {"location": "", "temperature": float('inf')},
            "windiest": {"location": "", "wind_speed": 0},
            "most_humid": {"location": "", "humidity": 0}
        }
    }
    
    for location in locations:
        try:
            weather_result = get_current_weather(location, units, language)
            
            if "error" in weather_result:
                comparison_data["locations"].append({
                    "location": location,
                    "error": weather_result["error"]
                })
                continue
            
            current_weather = weather_result["current_weather"]
            location_info = weather_result["location"]
            
            # Extract numeric temperature for comparison
            temp_str = current_weather["temperature"].replace("°C", "").replace("°F", "").replace("°K", "")
            try:
                temperature = float(temp_str)
            except ValueError:
                temperature = None
            
            location_data = {
                "location": location_info["name"],
                "country": location_info["country"],
                "temperature": current_weather["temperature"],
                "description": current_weather["description"],
                "feels_like": current_weather["feels_like"],
                "humidity": current_weather["humidity"],
                "wind": current_weather["wind"],
                "pressure": current_weather["pressure"],
                "local_time": location_info["local_time"]
            }
            
            comparison_data["locations"].append(location_data)
            
            # Update summary statistics
            if temperature is not None:
                if temperature > comparison_data["summary"]["hottest"]["temperature"]:
                    comparison_data["summary"]["hottest"] = {
                        "location": location_info["name"],
                        "temperature": temperature
                    }
                
                if temperature < comparison_data["summary"]["coldest"]["temperature"]:
                    comparison_data["summary"]["coldest"] = {
                        "location": location_info["name"],
                        "temperature": temperature
                    }
            
            # Extract humidity for comparison
            humidity_str = current_weather["humidity"].replace("%", "")
            try:
                humidity = float(humidity_str)
                if humidity > comparison_data["summary"]["most_humid"]["humidity"]:
                    comparison_data["summary"]["most_humid"] = {
                        "location": location_info["name"],
                        "humidity": humidity
                    }
            except ValueError:
                pass
            
            # Extract wind speed for comparison
            wind_parts = current_weather["wind"].split()
            if wind_parts:
                try:
                    wind_speed = float(wind_parts[0])
                    if wind_speed > comparison_data["summary"]["windiest"]["wind_speed"]:
                        comparison_data["summary"]["windiest"] = {
                            "location": location_info["name"],
                            "wind_speed": wind_speed
                        }
                except ValueError:
                    pass
                    
        except Exception as e:
            comparison_data["locations"].append({
                "location": location,
                "error": f"Failed to get weather data: {str(e)}"
            })
    
    return comparison_data

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
                        metadata = data.get('search_metadata', {})
                        location = data.get('location', {})
                        searches.append({
                            'search_id': search_id,
                            'search_type': metadata.get('search_type', 'unknown'),
                            'location': f"{location.get('name', 'N/A')}, {location.get('country', 'N/A')}",
                            'query': metadata.get('location_query', 'N/A'),
                            'search_time': metadata.get('search_timestamp', 'N/A'),
                            'units': metadata.get('units', 'N/A')
                        })
                except (json.JSONDecodeError, KeyError):
                    continue
    
    content = "# Weather Searches\n\n"
    if searches:
        content += f"Total searches: {len(searches)}\n\n"
        
        # Group by search type
        search_types = {}
        for search in searches:
            search_type = search['search_type']
            if search_type not in search_types:
                search_types[search_type] = []
            search_types[search_type].append(search)
        
        for search_type, type_searches in search_types.items():
            content += f"## {search_type.title()} Weather Searches\n\n"
            for search in type_searches:
                content += f"### {search['search_id']}\n"
                content += f"- **Location**: {search['location']}\n"
                content += f"- **Query**: {search['query']}\n"
                content += f"- **Units**: {search['units']}\n"
                content += f"- **Search Time**: {search['search_time']}\n\n"
                content += "---\n\n"
    else:
        content += "No weather searches found.\n\n"
        content += "Use the weather search tools to get weather data:\n"
        content += "- `get_current_weather()` for current weather\n"
        content += "- `get_weather_forecast()` for weather forecasts\n"
        content += "- `get_historical_weather()` for historical data\n"
    
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
        
        metadata = weather_data.get('search_metadata', {})
        location = weather_data.get('location', {})
        current = weather_data.get('current', {})
        forecast = weather_data.get('forecast', {})
        historical = weather_data.get('historical', {})
        
        content = f"# Weather Search: {search_id}\n\n"
        
        # Search Details
        content += f"## Search Details\n"
        content += f"- **Type**: {metadata.get('search_type', 'unknown').title()}\n"
        content += f"- **Location Query**: {metadata.get('location_query', 'N/A')}\n"
        content += f"- **Units**: {metadata.get('units', 'N/A')}\n"
        content += f"- **Language**: {metadata.get('language', 'N/A')}\n"
        content += f"- **Search Time**: {metadata.get('search_timestamp', 'N/A')}\n\n"
        
        # Location Information
        if location:
            content += f"## Location Information\n"
            content += f"- **Name**: {location.get('name', 'N/A')}\n"
            content += f"- **Country**: {location.get('country', 'N/A')}\n"
            content += f"- **Region**: {location.get('region', 'N/A')}\n"
            content += f"- **Coordinates**: {location.get('lat', 'N/A')}, {location.get('lon', 'N/A')}\n"
            content += f"- **Timezone**: {location.get('timezone_id', 'N/A')}\n"
            if location.get('localtime'):
                content += f"- **Local Time**: {location['localtime']}\n"
            content += "\n"
        
        # Current Weather
        if current:
            content += f"## Current Weather\n"
            content += f"- **Temperature**: {current.get('temperature', 'N/A')}°\n"
            content += f"- **Feels Like**: {current.get('feelslike', 'N/A')}°\n"
            content += f"- **Condition**: {', '.join(current.get('weather_descriptions', ['N/A']))}\n"
            content += f"- **Humidity**: {current.get('humidity', 'N/A')}%\n"
            content += f"- **Wind**: {current.get('wind_speed', 'N/A')} {current.get('wind_dir', '')}\n"
            content += f"- **Pressure**: {current.get('pressure', 'N/A')} mb\n"
            content += f"- **Visibility**: {current.get('visibility', 'N/A')}\n"
            content += f"- **UV Index**: {current.get('uv_index', 'N/A')}\n"
            content += f"- **Cloud Cover**: {current.get('cloudcover', 'N/A')}%\n"
            
            # Air Quality
            if current.get('air_quality'):
                aq = current['air_quality']
                content += f"\n### Air Quality\n"
                content += f"- **US EPA Index**: {aq.get('us-epa-index', 'N/A')}\n"
                content += f"- **UK DEFRA Index**: {aq.get('gb-defra-index', 'N/A')}\n"
                content += f"- **CO**: {aq.get('co', 'N/A')} μg/m³\n"
                content += f"- **NO2**: {aq.get('no2', 'N/A')} μg/m³\n"
                content += f"- **O3**: {aq.get('o3', 'N/A')} μg/m³\n"
                content += f"- **PM2.5**: {aq.get('pm2_5', 'N/A')} μg/m³\n"
            
            content += "\n"
        
        # Forecast Data
        if forecast:
            content += f"## Weather Forecast ({len(forecast)} days)\n\n"
            for date, day_data in list(forecast.items())[:3]:  # Show first 3 days
                content += f"### {date}\n"
                content += f"- **Temperature**: {day_data.get('mintemp', 'N/A')}° - {day_data.get('maxtemp', 'N/A')}° (avg: {day_data.get('avgtemp', 'N/A')}°)\n"
                content += f"- **UV Index**: {day_data.get('uv_index', 'N/A')}\n"
                content += f"- **Sun Hours**: {day_data.get('sunhour', 'N/A')}\n"
                
                if day_data.get('astro'):
                    astro = day_data['astro']
                    content += f"- **Sunrise**: {astro.get('sunrise', 'N/A')}\n"
                    content += f"- **Sunset**: {astro.get('sunset', 'N/A')}\n"
                    content += f"- **Moon Phase**: {astro.get('moon_phase', 'N/A')}\n"
                
                content += "\n"
            
            if len(forecast) > 3:
                content += f"... and {len(forecast) - 3} more days\n\n"
        
        # Historical Data
        if historical:
            content += f"## Historical Weather ({len(historical)} days)\n\n"
            for date, day_data in list(historical.items())[:3]:  # Show first 3 days
                content += f"### {date}\n"
                content += f"- **Temperature**: {day_data.get('mintemp', 'N/A')}° - {day_data.get('maxtemp', 'N/A')}° (avg: {day_data.get('avgtemp', 'N/A')}°)\n"
                content += f"- **UV Index**: {day_data.get('uv_index', 'N/A')}\n"
                content += f"- **Sun Hours**: {day_data.get('sunhour', 'N/A')}\n"
                content += f"- **Total Snow**: {day_data.get('totalsnow', 'N/A')}\n"
                content += "\n"
            
            if len(historical) > 3:
                content += f"... and {len(historical) - 3} more days\n\n"
        
        return content
        
    except json.JSONDecodeError:
        return f"# Error\n\nCorrupted weather data for search ID: {search_id}"

@mcp.prompt()
def weather_analysis_prompt(
    location: str,
    analysis_type: str = "current",
    days: int = 5,
    units: str = "m",
    include_comparison: bool = False,
    comparison_locations: List[str] = []
) -> str:
    """Generate a comprehensive weather analysis prompt for Claude."""
    
    unit_display = "Celsius" if units == "m" else "Fahrenheit" if units == "f" else "Kelvin"
    
    prompt = f"""Provide a comprehensive weather analysis for {location} using the Weather Search MCP tools."""
    
    if analysis_type == "current":
        prompt += f"""

**Current Weather Analysis**

1. **Get Current Weather**: Use get_current_weather('{location}', '{units}') to retrieve current conditions
2. **Detailed Analysis**: Provide insights on:
   - Current temperature and feels-like temperature in {unit_display}
   - Weather conditions and visibility
   - Wind patterns and direction
   - Humidity and pressure analysis
   - Air quality assessment (if available)
   - UV index and sun exposure recommendations
   - Astronomical information (sunrise, sunset, moon phase)

3. **Location Context**: Include information about:
   - Exact location coordinates and timezone
   - Regional weather patterns
   - Seasonal considerations for this time of year"""

    elif analysis_type == "forecast":
        prompt += f"""

**Weather Forecast Analysis**

1. **Get Forecast Data**: Use get_weather_forecast('{location}', {days}, True, '{units}') for detailed forecast
2. **Forecast Analysis**: Provide insights on:
   - Daily temperature trends and ranges
   - Weather pattern changes over the {days}-day period  
   - Precipitation probability and amounts
   - Wind speed and direction changes
   - Pressure trends and weather system movements
   - Best days for outdoor activities
   - Days to avoid due to severe weather

3. **Planning Recommendations**: Include advice for:
   - Clothing and preparation suggestions
   - Travel and outdoor activity planning
   - Weather alerts or warnings to watch for"""

    elif analysis_type == "historical":
        today = datetime.now()
        historical_date = (today - timedelta(days=365)).strftime('%Y-%m-%d')
        prompt += f"""

**Historical Weather Analysis**

1. **Get Historical Data**: Use get_historical_weather('{location}', '{historical_date}', None, True, '{units}') 
2. **Historical Analysis**: Compare with current conditions:
   - Temperature trends year-over-year
   - Seasonal weather patterns
   - Climate change indicators
   - Historical weather extremes
   - Typical weather for this time of year"""

    if include_comparison and comparison_locations:
        locations_str = ', '.join(comparison_locations)
        prompt += f"""

4. **Weather Comparison**: Use compare_weather(['{location}'] + {comparison_locations}, '{units}') to compare:
   - Temperature differences across locations
   - Regional weather variations
   - Climate zone comparisons
   - Travel destination weather analysis"""

    prompt += f"""

**Presentation Format**:
- Use clear headings and bullet points
- Include specific numerical data with units
- Provide practical recommendations
- Highlight any notable weather conditions or warnings
- Create easy-to-scan summaries for key information

**Additional Tools Available**:
- search_locations() for location disambiguation
- get_weather_details() for accessing saved searches
- Multiple weather searches for trend analysis

Focus on providing actionable insights that help with daily planning, travel decisions, and weather awareness."""

    return prompt

@mcp.prompt()
def weather_comparison_prompt(
    locations: List[str],
    comparison_focus: str = "general",
    units: str = "m"
) -> str:
    """Generate a prompt for detailed weather comparison across multiple locations."""
    
    locations_str = ', '.join(locations)
    
    return f"""Compare current weather conditions across multiple locations: {locations_str}

**Weather Comparison Analysis**

1. **Data Collection**: 
   - Use compare_weather({locations}, '{units}') to get comparative data
   - Use get_current_weather() for each location to get detailed individual data

2. **Comparison Analysis Focus - {comparison_focus.title()}**:"""

    if comparison_focus == "travel":
        prompt += """
   - Best weather for travel and sightseeing  
   - Clothing and packing recommendations for each location
   - Outdoor activity suitability
   - Transportation weather considerations
   - UV protection needs"""
    
    elif comparison_focus == "business":
        prompt += """
   - Weather impact on business operations
   - Commuting and transportation conditions
   - Office comfort and energy considerations
   - Meeting and event planning implications
   - Supply chain weather effects"""
    
    elif comparison_focus == "agriculture":
        prompt += """
   - Growing conditions and crop suitability
   - Irrigation and water management needs
   - Pest and disease pressure indicators
   - Harvest timing considerations
   - Livestock comfort and care requirements"""
    
    else:  # general
        prompt += """
   - Temperature and comfort level comparison
   - Precipitation and weather condition differences
   - Air quality variations
   - Wind and atmospheric pressure analysis
   - Seasonal weather pattern comparison"""

    prompt += f"""

3. **Detailed Comparison Table**: Create a side-by-side comparison showing:
   - Location name and local time
   - Temperature (actual and feels-like)
   - Weather conditions and descriptions
   - Humidity and precipitation
   - Wind speed and direction
   - Pressure and visibility
   - Air quality indices (if available)

4. **Key Insights**:
   - Identify the location with the most favorable conditions
   - Highlight significant weather differences
   - Note any extreme or unusual weather patterns
   - Provide recommendations based on the comparison focus

5. **Recommendations**:
   - Best location for current conditions
   - Locations to avoid due to weather
   - Timing recommendations for activities
   - Follow-up weather monitoring suggestions

**Format Requirements**:
- Use clear tables and bullet points
- Include all relevant units of measurement
- Highlight key differences and similarities
- Provide actionable conclusions
- Include data timestamps for reference

Make the comparison practical and decision-focused, helping users choose between locations or plan activities accordingly."""

    return prompt

if __name__ == "__main__":
    # Initialize and run the server
    mcp.run(transport='stdio')