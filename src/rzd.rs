use std::collections::HashMap;
use std::default::Default;
use std::sync::Arc;
use std::time::Duration;

use async_recursion::async_recursion;
use fake_useragent::{Browsers, UserAgents, UserAgentsBuilder};
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use reqwest::StatusCode;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;
use tokio::sync::Mutex;

const BASE_API_URL: &str = "https://ticket.rzd.ru/api/v1";
const BASE_PASS_URL: &str = "https://pass.rzd.ru";
const ROUTES_LAYER: usize = 5827;
const CARRIEAGES_LAYER: usize = 5764;

fn places_deserialize<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let str_sequence = String::deserialize(deserializer)?;
    Ok(str_sequence
        .split(',')
        .map(|item| item.to_owned())
        .collect())
}
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
pub struct GetRZDTrainsCarriagesCars {
    #[serde(deserialize_with = "places_deserialize")]
    pub(crate) places: Vec<String>,
    pub(crate) cnumber: String,
    #[serde(rename = "type")]
    pub(crate) _type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetRZDTrainsCarriagesListResponse {
    pub(crate) cars: Vec<GetRZDTrainsCarriagesCars>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetRZDTrainsCarriagesResponse {
    pub(crate) lst: Vec<GetRZDTrainsCarriagesListResponse>,
}

pub struct RZDApi {
    ua: Mutex<UserAgents>,
}
impl RZDApi {
    #[must_use]
    pub fn new() -> Arc<Self> {
        let user_agents = UserAgentsBuilder::new()
            .set_browsers(Browsers::new().set_chrome().set_edge().set_firefox())
            .cache(false)
            .build();
        Arc::new(Self {
            ua: Mutex::from(user_agents),
        })
    }
    #[async_recursion]
    pub async fn get_rzd_point_codes(
        &self,
        part_or_full_name: String,
        retry_counter: isize,
    ) -> Result<Vec<GetRZDPointCodes>, String> {
        if retry_counter == -1 {
            return Err("Error on fetching info from rzd".to_string());
        }
        let client = reqwest::ClientBuilder::new()
            .cookie_store(true)
            .user_agent(self.ua.lock().await.random())
            .build()
            .unwrap();

        let query: String = part_or_full_name.clone();
        let query_params = vec![
            ("GroupResults", "true"),
            ("RailwaySortPriority", "true"),
            ("MergeSuburban", "true"),
            ("Query", &query),
            ("Language", "ru"),
            ("TransportType", "rail"),
        ];
        let url = reqwest::Url::parse_with_params(
            &(BASE_API_URL.to_owned() + "/suggests"),
            &query_params,
        )
        .unwrap();
        let result = client
            .get(url)
            .header(ACCEPT, "application/json")
            .send()
            .await;

        if result.is_err() {
            return self
                .get_rzd_point_codes(part_or_full_name.clone(), retry_counter - 1)
                .await;
        }

        let r = result.unwrap();
        if r.status() != 200 {
            log::warn!("got {} from rzd in function get_rzd_point_codes with params: part_or_full_name = {}, retry_counter = {}", r.status(), part_or_full_name, retry_counter);
            if r.status().as_u16() == StatusCode::FORBIDDEN.as_u16() {
                return self
                    .get_rzd_point_codes(part_or_full_name.clone(), retry_counter - 1)
                    .await;
            }
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
    #[async_recursion]
    pub async fn get_trains_from_rzd(
        &self,
        point_from: String,
        point_to: String,
        date: String,
        retry_counter: isize,
    ) -> Result<GetRZDTrainsResponse, String> {
        if retry_counter == -1 {
            return Err("Error on fetching info from rzd".to_string());
        }
        let client = reqwest::ClientBuilder::new()
            .cookie_store(true)
            .user_agent(self.ua.lock().await.random())
            .build()
            .unwrap();
        let query_params = vec![
            ("layer_id", ROUTES_LAYER.to_string()),
            ("dir", "0".to_string()),
            ("tfl", "1".to_string()),
            ("checkSeats", "1".to_string()),
            ("code0", point_from.clone()),
            ("code1", point_to.clone()),
            ("dt0", date.clone()),
            ("md", "0".to_string()),
        ];
        let url = reqwest::Url::parse_with_params(
            &(BASE_PASS_URL.to_owned() + "/timetable/public/ru"),
            &query_params,
        )
        .unwrap();
        let result = client
            .get(url)
            .header(ACCEPT, "application/json")
            .send()
            .await;
        if result.is_err() {
            return self
                .get_trains_from_rzd(
                    point_from.clone(),
                    point_to.clone(),
                    date.clone(),
                    retry_counter - 1,
                )
                .await;
        }

        let r = result.unwrap();
        if r.status() != 200 {
            log::warn!("got {} from rzd in function get_trains_from_rzd with params: point_from = {}, point_to = {}, date = {}, retry_counter = {}", r.status(), point_from, point_to, date, retry_counter);
            if r.status().as_u16() == StatusCode::FORBIDDEN.as_u16() {
                return self
                    .get_trains_from_rzd(
                        point_from.clone(),
                        point_to.clone(),
                        date.clone(),
                        retry_counter - 1,
                    )
                    .await;
            }
            return Err(format!("Invalid response code from rzd {}", r.status()));
        }
        let response_string_result = r.text().await;
        if let Err(err) = response_string_result {
            return Err(format!("Error on getting response bytes {}", err));
        }
        let response_string = response_string_result.unwrap();
        let rid_response_result = serde_json::from_str::<HashMap<String, serde_json::Value>>(
            response_string
                .as_str()
                .trim_matches(|c: char| c.is_whitespace() || c == '\"'),
        );

        if let Err(err) = rid_response_result {
            return Err(format!("Error on deserialize json {}", err));
        }

        let rid_response = rid_response_result.unwrap();
        if rid_response.get("RID").is_none() {
            if rid_response
                .get("result")
                .unwrap_or(&json!(""))
                .eq(&json!("FAIL"))
            {
                return self
                    .get_trains_from_rzd(
                        point_from.clone(),
                        point_to.clone(),
                        date.clone(),
                        retry_counter - 1,
                    )
                    .await;
            }
            return match serde_json::from_str::<GetRZDTrainsResponse>(
                response_string
                    .as_str()
                    .trim_matches(|c: char| c.is_whitespace() || c == '\"'),
            ) {
                Ok(v) => return Ok(v),
                Err(err) => Err(format!("Error on deserialize json {}", err)),
            };
        }

        let mut c = 0;
        let rid = rid_response.get("RID").unwrap();

        loop {
            let query_params = vec![("layer_id", ROUTES_LAYER.to_string())];
            let url = reqwest::Url::parse_with_params(
                &(BASE_PASS_URL.to_owned() + "/timetable/public/ru"),
                &query_params,
            )
            .unwrap();

            let result = client
                .post(url)
                .header(ACCEPT, "application/json")
                .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(format!("rid={rid}"))
                .send()
                .await;

            if result.is_err() {
                return self
                    .get_trains_from_rzd(
                        point_from.clone(),
                        point_to.clone(),
                        date.clone(),
                        retry_counter - 1,
                    )
                    .await;
            }

            let r = result.unwrap();
            if r.status() != 200 {
                log::warn!("got {} from rzd in function get_trains_from_rzd with params: point_from = {}, point_to = {}, date = {}, retry_counter = {}", r.status(), point_from, point_to, date, retry_counter);
                if r.status().as_u16() == StatusCode::FORBIDDEN.as_u16() {
                    return self
                        .get_trains_from_rzd(
                            point_from.clone(),
                            point_to.clone(),
                            date.clone(),
                            retry_counter - 1,
                        )
                        .await;
                }
                return Err(format!("Invalid response code from rzd {}", r.status()));
            }

            let response_string_result = r.text().await;
            if let Err(err) = response_string_result {
                return Err(format!("Error on getting response bytes {}", err));
            }
            let response_string = response_string_result.unwrap();
            let rid_response_result = serde_json::from_str::<HashMap<String, serde_json::Value>>(
                response_string
                    .as_str()
                    .trim_matches(|c: char| c.is_whitespace() || c == '\"'),
            );

            if let Err(err) = rid_response_result {
                return Err(format!("Error on deserialize json {}", err));
            }

            let rid_response = rid_response_result.unwrap();
            if rid_response.get("RID").is_none() {
                if rid_response
                    .get("result")
                    .unwrap_or(&json!("".to_string()))
                    .eq(&json!("FAIL"))
                {
                    return self
                        .get_trains_from_rzd(
                            point_from.clone(),
                            point_to.clone(),
                            date.clone(),
                            retry_counter - 1,
                        )
                        .await;
                }
                return match serde_json::from_str::<GetRZDTrainsResponse>(
                    response_string
                        .as_str()
                        .trim_matches(|c: char| c.is_whitespace() || c == '\"'),
                ) {
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

    #[async_recursion]
    pub async fn get_trains_carriages_from_rzd(
        &self,
        point_from: String,
        point_to: String,
        dt0: String,
        time0: String,
        tnum0: String,
        retry_counter: isize,
    ) -> Result<GetRZDTrainsCarriagesResponse, String> {
        if retry_counter == -1 {
            return Err("Error on fetching info from rzd".to_string());
        }
        let client = reqwest::ClientBuilder::new()
            .cookie_store(true)
            .user_agent(self.ua.lock().await.random())
            .build()
            .unwrap();
        let query_params = vec![
            ("layer_id", CARRIEAGES_LAYER.to_string()),
            ("dir", "0".to_string()),
            ("code0", point_from.clone()),
            ("code1", point_to.clone()),
            ("dt0", dt0.clone()),
            ("time0", time0.clone()),
            ("tnum0", tnum0.clone()),
        ];
        let url = reqwest::Url::parse_with_params(
            &(BASE_PASS_URL.to_owned() + "/timetable/public/ru"),
            &query_params,
        )
        .unwrap();
        let result = client
            .get(url)
            .header(ACCEPT, "application/json")
            .send()
            .await;
        if result.is_err() {
            return self
                .get_trains_carriages_from_rzd(
                    point_from.clone(),
                    point_to.clone(),
                    dt0.clone(),
                    time0.clone(),
                    tnum0.clone(),
                    retry_counter - 1,
                )
                .await;
        }

        let r = result.unwrap();
        if r.status() != 200 {
            log::warn!("got {} from rzd in function get_trains_carriages_from_rzd with params: point_from = {}, point_to = {}, dt0 = {}, time0 = {}, tnum0 = {}, retry_counter = {}", r.status(), point_from, point_to, dt0, time0, tnum0, retry_counter);
            if r.status().as_u16() == StatusCode::FORBIDDEN.as_u16() {
                return self
                    .get_trains_carriages_from_rzd(
                        point_from.clone(),
                        point_to.clone(),
                        dt0.clone(),
                        time0.clone(),
                        tnum0.clone(),
                        retry_counter - 1,
                    )
                    .await;
            }
            return Err(format!("Invalid response code from rzd {}", r.status()));
        }
        let response_string_result = r.text().await;
        if let Err(err) = response_string_result {
            return Err(format!("Error on getting response bytes {}", err));
        }
        let response_string = response_string_result.unwrap();
        let rid_response_result = serde_json::from_str::<HashMap<String, serde_json::Value>>(
            response_string
                .as_str()
                .trim_matches(|c: char| c.is_whitespace() || c == '\"'),
        );

        if let Err(err) = rid_response_result {
            return Err(format!("Error on deserialize json {}", err));
        }

        let rid_response = rid_response_result.unwrap();
        if rid_response.get("RID").is_none() {
            if rid_response
                .get("result")
                .unwrap_or(&json!(""))
                .eq(&json!("FAIL"))
            {
                return self
                    .get_trains_carriages_from_rzd(
                        point_from.clone(),
                        point_to.clone(),
                        dt0.clone(),
                        time0.clone(),
                        tnum0.clone(),
                        retry_counter - 1,
                    )
                    .await;
            }
            return match serde_json::from_str::<GetRZDTrainsCarriagesResponse>(
                response_string
                    .as_str()
                    .trim_matches(|c: char| c.is_whitespace() || c == '\"'),
            ) {
                Ok(v) => return Ok(v),
                Err(err) => Err(format!("Error on deserialize json {}", err)),
            };
        }

        let mut c = 0;
        let rid = rid_response.get("RID").unwrap();

        loop {
            let result = client
                .post(&(BASE_PASS_URL.to_owned() + "/timetable/public/ru"))
                .header(ACCEPT, "application/json")
                .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(format!("rid={rid}&layer_id={CARRIEAGES_LAYER}&dir=0&code0={point_from}&code1={point_to}&dt0={dt0}&time0={time0}&tnum0={tnum0}"))
                .send()
                .await;

            if result.is_err() {
                return self
                    .get_trains_carriages_from_rzd(
                        point_from.clone(),
                        point_to.clone(),
                        dt0.clone(),
                        time0.clone(),
                        tnum0.clone(),
                        retry_counter - 1,
                    )
                    .await;
            }

            let r = result.unwrap();
            if r.status() != 200 {
                log::warn!("got {} from rzd in function get_trains_carriages_from_rzd with params: point_from = {}, point_to = {}, dt0 = {}, time0 = {}, tnum0 = {}, retry_counter = {}", r.status(), point_from, point_to, dt0, time0, tnum0, retry_counter);
                if r.status().as_u16() == StatusCode::FORBIDDEN.as_u16() {
                    return self
                        .get_trains_carriages_from_rzd(
                            point_from.clone(),
                            point_to.clone(),
                            dt0.clone(),
                            time0.clone(),
                            tnum0.clone(),
                            retry_counter - 1,
                        )
                        .await;
                }
                return Err(format!("Invalid response code from rzd {}", r.status()));
            }

            let response_string_result = r.text().await;
            if let Err(err) = response_string_result {
                return Err(format!("Error on getting response bytes {}", err));
            }
            let response_string = response_string_result.unwrap();
            let rid_response_result = serde_json::from_str::<HashMap<String, serde_json::Value>>(
                response_string
                    .as_str()
                    .trim_matches(|c: char| c.is_whitespace() || c == '\"'),
            );

            if let Err(err) = rid_response_result {
                return Err(format!("Error on deserialize json {}", err));
            }

            let rid_response = rid_response_result.unwrap();
            if rid_response.get("RID").is_none() {
                if rid_response
                    .get("result")
                    .unwrap_or(&json!("".to_string()))
                    .eq(&json!("FAIL"))
                {
                    return self
                        .get_trains_carriages_from_rzd(
                            point_from.clone(),
                            point_to.clone(),
                            dt0.clone(),
                            time0.clone(),
                            tnum0.clone(),
                            retry_counter - 1,
                        )
                        .await;
                }
                return match serde_json::from_str::<GetRZDTrainsCarriagesResponse>(
                    response_string
                        .as_str()
                        .trim_matches(|c: char| c.is_whitespace() || c == '\"'),
                ) {
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
}

//
