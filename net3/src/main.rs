use std::error::Error;
use std::fmt;
use reqwest::Client;
use serde::Deserialize;
use tokio::{io, join};
use tokio::io::AsyncBufReadExt;
use futures::future::try_join_all;

#[derive(Deserialize, Debug)]
struct Weather {
    id: u64,
    main: String,
    description: String,
    icon: String,
}

#[derive(Deserialize, Debug)]
struct Main {
    temp: f64,
    feels_like: f64,
    temp_min: f64,
    temp_max: f64,
    pressure: u64,
    humidity: u64,
    sea_level: u64,
    grnd_level: u64,
}

#[derive(Deserialize, Debug)]
struct Wind {
    speed: f64,
    deg: u64,
    gust: f64,
}

#[derive(Deserialize, Debug)]
struct Rain {
    #[serde(rename = "1h")]
    one_hour: f64,
}

#[derive(Deserialize, Debug)]
struct Clouds {
    all: u64,
}


#[derive(Deserialize)]
struct WeatherData {
    weather: Vec<Weather>,
    base: String,
    main: Main,
    visibility: u64,
    wind: Wind,
    rain: Option<Rain>,
    clouds: Clouds,
    dt: u64,
    timezone: u64,
    id: u64,
    name: String,
    cod: u64,
}

#[derive(Deserialize)]
struct GeocodingResponse {
    hits: Vec<GeocodingLocation>,
}

#[derive(Deserialize)]
struct GeocodingLocation {
    point: Point,
    osm_id: u64,
    osm_type: String,
    osm_key: String,
    osm_value: String,
    name: String,
    country: String,
    city: Option<String>,
    state: Option<String>,
    street: Option<String>,
    housenumber: Option<String>,
    postcode: Option<String>,
}

#[derive(Deserialize, Debug)]
struct Point {
    lat: f64, // Latitude
    lng: f64, // Longitude
}

#[derive(Deserialize)]
pub struct FeatureCollection {
    #[serde(rename = "type")]
    pub type_: String,
    pub features: Vec<Feature>,
}

#[derive(Deserialize)]
pub struct Feature {
    #[serde(rename = "type")]
    pub type_: String,
    pub id: String,
    pub geometry: Geometry,
    pub properties: Properties,
}

#[derive(Deserialize, Debug)]
pub struct Geometry {
    #[serde(rename = "type")]
    pub type_: String,
    pub coordinates: Vec<f64>,
}

#[derive(Deserialize, Debug)]
pub struct Properties {
    pub xid: String,
    pub name: String,
    pub dist: Option<f64>,
    pub osm: Option<String>,
    pub kinds: Option<String>,
    pub wikidata: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PlaceDescription {
    pub xid: String,
    pub name: String,
    pub osm: Option<String>,
    pub wikidata: Option<String>,
    pub rate: Option<String>,
    pub image: Option<String>,
    pub wikipedia: Option<String>,
    pub kinds: Option<String>,
    pub sources: Option<Sources>,
    pub bbox: Option<Bbox>,
    pub point: Option<Point2>,
    pub otm: Option<String>,
    pub info: Option<Info>,
}

#[derive(Debug, Deserialize)]
pub struct Sources {
    pub geometry: String,
    pub attributes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Bbox {
    pub lat_max: f64,
    pub lat_min: f64,
    pub lon_max: f64,
    pub lon_min: f64,
}

#[derive(Debug, Deserialize)]
pub struct Point2 {
    pub lon: f64,
    pub lat: f64,
}

#[derive(Debug, Deserialize)]
pub struct Info {
    pub descr: String,
    pub image: String,
    pub img_width: i32,
}

// Implementing Display for WeatherData
impl fmt::Display for WeatherData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WeatherData {{\n")?;
        write!(f, "  weather: {:?},\n", self.weather)?;
        write!(f, "  base: {},\n", self.base)?;
        write!(f, "  main: {:?},\n", self.main)?;
        write!(f, "  visibility: {},\n", self.visibility)?;
        write!(f, "  wind: {:?},\n", self.wind)?;
        write!(f, "  rain: {:?},\n", self.rain)?;
        write!(f, "  clouds: {:?},\n", self.clouds)?;
        write!(f, "  dt: {},\n", self.dt)?;
        write!(f, "  timezone: {},\n", self.timezone)?;
        write!(f, "  id: {},\n", self.id)?;
        write!(f, "  name: {},\n", self.name)?;
        write!(f, "  cod: {},\n", self.cod)?;
        write!(f, "}}")
    }
}

// Implementing Display for GeocodingLocation
impl fmt::Display for GeocodingLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GeocodingLocation {{\n")?;
        write!(f, "  point: {:?},\n", self.point)?;
        write!(f, "  osm_id: {},\n", self.osm_id)?;
        write!(f, "  osm_type: {},\n", self.osm_type)?;
        write!(f, "  osm_key: {},\n", self.osm_key)?;
        write!(f, "  osm_value: {},\n", self.osm_value)?;
        write!(f, "  name: {},\n", self.name)?;
        write!(f, "  country: {},\n", self.country)?;
        write!(f, "  city: {:?},\n", self.city)?;
        write!(f, "  state: {:?},\n", self.state)?;
        write!(f, "  street: {:?},\n", self.street)?;
        write!(f, "  housenumber: {:?},\n", self.housenumber)?;
        write!(f, "  postcode: {:?},\n", self.postcode)?;
        write!(f, "}}")
    }
}

// Implementing Display for Feature
impl fmt::Display for Feature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Feature {{\n")?;
        write!(f, "  type_: {},\n", self.type_)?;
        write!(f, "  id: {},\n", self.id)?;
        write!(f, "  geometry: {:?},\n", self.geometry)?;
        write!(f, "  properties: {:?},\n", self.properties)?;
        write!(f, "}}")
    }
}

impl fmt::Display for Properties {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "xid: {}, name: {}, dist: {:?}, osm: {:?}, kinds: {:?}, wikidata: {:?}",
            self.xid, self.name, self.dist, self.osm, self.kinds, self.wikidata
        )
    }
}

impl fmt::Display for PlaceDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PlaceDescription {{\n  xid: {}\n  name: {}\n  osm: {:?}\n  wikidata: {:?}\n  rate: {:?}\n  image: {:?}\n  wikipedia: {:?}\n  kinds: {:?}\n  sources: {:?}\n  bbox: {:?}\n  point: {:?}\n  otm: {:?}\n  info: {:?}\n}}",
               self.xid, self.name, self.osm, self.wikidata, self.rate, self.image, self.wikipedia, self.kinds, self.sources, self.bbox, self.point, self.otm, self.info)
    }
}

async fn get_locations(query: &str, key: &str) -> Result<Vec<GeocodingLocation>, Box<dyn Error>> {
    let client = Client::new();
    let url = format!("https://graphhopper.com/api/1/geocode?q={query}&limit=10&key={key}");
    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        panic!("bad request {:?}", response);
    }
    // Deserialize the JSON response using serde_json
    let response_text = response.text().await?;
    let locations: GeocodingResponse = serde_json::from_str(&response_text)?;
    Ok(locations.hits)
}

async fn get_features(point: &Point, key: &str) -> Result<Vec<Feature>, Box<dyn Error>> {
    let client = Client::new();
    let url = format!("https://api.opentripmap.com/0.1/en/places/radius?radius=10000&lon={}&lat={}&apikey={}", point.lng, point.lat, key);
    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        panic!("bad request {:?}", response);
    }
    // Deserialize the JSON response using serde_json
    let response_text = response.text().await?;
    let features: FeatureCollection = serde_json::from_str(&response_text)?;
    Ok(features.features)
}

async fn get_description(xid: String, key: &str) -> Result<PlaceDescription, Box<dyn Error>>{
    let client = Client::new();
    let url = format!("https://api.opentripmap.com/0.1/en/places/xid/{xid}?apikey={key}");
    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        panic!("bad request {:?}", response);
    }
    // Deserialize the JSON response using serde_json
    let response_text = response.text().await?;
    let description: PlaceDescription = serde_json::from_str(&response_text)?;
    Ok(description)
}

async fn get_descriptions(places: &Vec<Feature>, key: &str) -> Result<Vec<PlaceDescription>, Box<dyn Error>>{
    let mut descriptions = vec!();
    for feature in places {
        let description = get_description(feature.id.clone(), &key);
        descriptions.push(description);
    }
    return try_join_all(descriptions).await;
}


async fn get_weather(point: &Point, key: &str) -> Result<WeatherData, Box<dyn Error>> {
    let client = Client::new();
    let url = format!("https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&apikey={key}", point.lat, point.lng);
    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        panic!("bad request {:?}", response);
    }
    // Deserialize the JSON response using serde_json
    let response_text = response.text().await?;
    let weather: WeatherData = serde_json::from_str(&response_text)?;
    Ok(weather)
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let location_key = "6d667004-04ff-4410-8708-7b59da4a026d";
    let places_key = "5ae2e3f221c38a28845f05b6736e7fff0a03db995bb8e1f97679c4fb";
    let weather_key = "d8e9c15c0cc45295def227fb3338ab5f";

    let mut reader = io::BufReader::new(io::stdin());

    println!("Enter location:");
    let mut query = String::new();
    reader.read_line(&mut query).await?;

    // Get locations based on user input.
    let locations = get_locations(&query, &location_key).await?;

    // Choose one.
    locations.iter().for_each(|location| println!("{}", &location));
    println!("Select location number from 0 to {}", locations.len()-1);

    let mut input = String::new();
    reader.read_line(&mut input).await?;
    let index: usize = input.trim().parse()?;
    let selected_location: &GeocodingLocation = &locations[index];

    // Creating tasks for weather and places
    let weather_task = get_weather(&selected_location.point, &weather_key);
    let places_task = get_features(&selected_location.point, &places_key);

    let places = places_task.await?;

    let descriptions_task = get_descriptions(&places, places_key);


    // Waiting for all tasks to be completed.
    let (weather_res, descriptions_res) = join!(weather_task, descriptions_task);
    let weather = weather_res?;
    let descriptions = descriptions_res?;
    println!("Weather is: {}\n", weather);
    println!("Places of interest are: ");
    for (place, description) in places.iter().zip(descriptions) {
        println!("Place: {} with description {}\n\n", place, description);
    }
    Ok(())
}
