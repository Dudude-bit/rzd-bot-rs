use std::collections::HashMap;
use std::default::Default;
use std::string::String;
use std::thread::sleep;
use std::time::Duration;

use serde::{Deserialize, Serialize};

const BASE_API_URL: &str = "https://ticket.rzd.ru/api/v1";
const BASE_PASS_URL: &str = "https://pass.rzd.ru";
const ROUTES_LAYER: usize = 5827;
const CARRIEAGES_LAYER: usize = 5764;
const USER_AGENT: &str = "Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.6099.144 Mobile Safari/537.36";
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetRZDPointCodes {
    #[serde(rename = "expressCode")]
    pub(crate) code: String,
    pub(crate) name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetRZDTrainsCars {
    #[serde(rename = "type")]
    pub(crate) _type: String,

    #[serde(default = "bool::default", rename = "disabledPerson")]
    pub(crate) disabled_person: bool,

    #[serde(rename = "freeSeats")]
    pub(crate) free_seats: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetRZDTrains {
    pub(crate) cars: Vec<GetRZDTrainsCars>,
    pub(crate) number: String,
    pub(crate) date0: String,
    pub(crate) time0: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetRZDTrainsListResponse {
    pub(crate) list: Vec<GetRZDTrains>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetRZDTrainsResponse {
    pub(crate) tp: Vec<GetRZDTrainsListResponse>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RIDRequest {
    #[serde(rename = "RID")]
    rid: String,
}

pub async fn get_rzd_point_codes(
    part_or_full_name: String,
) -> Result<Vec<GetRZDPointCodes>, String> {
    let client = reqwest::ClientBuilder::new()
        .cookie_store(true)
        .build()
        .unwrap();

    let query: String = part_or_full_name.into();
    let query_params = vec![
        ("GroupResults", "true"),
        ("RailwaySortPriority", "true"),
        ("MergeSuburban", "true"),
        ("Query", &query),
        ("Language", "ru"),
        ("TransportType", "rail"),
    ];
    let url =
        reqwest::Url::parse_with_params(&(BASE_API_URL.to_owned() + "/suggests"), &query_params)
            .unwrap();
    let result = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/json")
        .send()
        .await;

    if let Err(err) = result {
        return Err(format!("Error on fetching info from rzd {}", err));
    }

    let r = result.unwrap();
    if r.status() != 200 {
        return Err(format!("Invalid response code from rzd {}", r.status()));
    }

    let json_response = r.json::<HashMap<String, serde_json::Value>>().await;
    if let Err(err) = json_response {
        return Err(format!("Error on deserialize json {}", err));
    }

    return match json_response.unwrap().get("city") {
        None => Err("cant get cities for given request from rzd".to_string()),
        Some(v) => {
            let cities_serialized = serde_json::from_value(v.clone()).unwrap();
            Ok(cities_serialized)
        }
    };
}

pub async fn get_trains_from_rzd(
    point_from: String,
    point_to: String,
    date: String,
) -> Result<GetRZDTrainsResponse, String> {
    let client = reqwest::ClientBuilder::new()
        .cookie_store(true)
        .build()
        .unwrap();
    let query_params = vec![
        ("layer_id", ROUTES_LAYER.to_string()),
        ("dir", "0".to_string()),
        ("tfl", "1".to_string()),
        ("checkSeats", "1".to_string()),
        ("code0", point_from),
        ("code1", point_to),
        ("dt0", date),
        ("md", "0".to_string()),
    ];
    let url = reqwest::Url::parse_with_params(
        &(BASE_PASS_URL.to_owned() + "/timetable/public/ru"),
        &query_params,
    )
    .unwrap();
    let result = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/json")
        .send()
        .await;
    if let Err(err) = result {
        return Err(format!("Error on fetching info from rzd {}", err));
    }

    let r = result.unwrap();
    if r.status() != 200 {
        return Err(format!("Invalid response code from rzd {}", r.status()));
    }
    let response_string_result = r.text().await;
    if let Err(err) = response_string_result {
        return Err(format!("Error on getting response bytes {}", err));
    }
    let response_string = response_string_result.unwrap();
    let rid_response_result =
        serde_json::from_str::<HashMap<String, serde_json::Value>>(response_string.as_str());

    if let Err(err) = rid_response_result {
        return Err(format!("Error on deserialize json {}", err));
    }

    let rid_response = rid_response_result.unwrap();
    if rid_response.get("RID").is_none() {
        if rid_response
            .get("result")
            .unwrap_or(&serde_json::Value::String("".to_string()))
            .eq(&serde_json::Value::String("FAIL".to_string()))
        {
            return Err("Error on fetching info from rzd".to_string());
        }
        return match serde_json::from_str::<GetRZDTrainsResponse>(response_string.as_str()) {
            Ok(v) => return Ok(v),
            Err(err) => Err(format!("Error on deserialize json {}", err)),
        };
    }

    let mut c = 0;
    let rid = rid_response.get("RID").unwrap().as_i64().unwrap();
    loop {
        let query_params = vec![("layer_id", ROUTES_LAYER.to_string())];
        let url = reqwest::Url::parse_with_params(
            &(BASE_PASS_URL.to_owned() + "/timetable/public/ru"),
            &query_params,
        )
        .unwrap();

        let result = client
            .post(url)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "application/json")
            .body(format!("rid={rid}"))
            .send()
            .await;

        if let Err(err) = result {
            return Err(format!("Error on fetching info from rzd {}", err));
        }

        let r = result.unwrap();
        if r.status() != 200 {
            return Err(format!("Invalid response code from rzd {}", r.status()));
        }

        let response_string_result = r.text().await;
        if let Err(err) = response_string_result {
            return Err(format!("Error on getting response bytes {}", err));
        }
        let response_string = response_string_result.unwrap();
        let rid_response_result =
            serde_json::from_str::<HashMap<String, serde_json::Value>>(response_string.as_str());

        if let Err(err) = rid_response_result {
            return Err(format!("Error on deserialize json {}", err));
        }

        let rid_response = rid_response_result.unwrap();
        if rid_response.get("RID").is_none() {
            if rid_response
                .get("result")
                .unwrap_or(&serde_json::Value::String("".to_string()))
                .eq(&serde_json::Value::String("FAIL".to_string()))
            {
                return Err("Error on fetching info from rzd".to_string());
            }
            return match serde_json::from_str::<GetRZDTrainsResponse>(response_string.as_str()) {
                Ok(v) => return Ok(v),
                Err(err) => Err(format!("Error on deserialize json {}", err)),
            };
        }
        c += 1;

        if c > 5 {
            return Err("Cant fetch data from rzd".to_string());
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
