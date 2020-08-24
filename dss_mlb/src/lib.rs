#![allow(non_snake_case)]

use chrono::{DateTime, Duration, Local, NaiveDate, TimeZone, Utc};
use hyper::client::HttpConnector;
use hyper::{Body, Client};
use hyper_tls::HttpsConnector;
use log::error;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Display;

const GAME_API: &str =
    "http://statsapi.mlb.com/api/v1/schedule?hydrate=game(content(editorial(recap))),decisions&sportId=1&date=";

/// A container for MLB game information over a range of dates.
#[derive(Debug, Deserialize, Serialize)]
struct MlbGameRange {
    dates: Vec<MlbGameDateInfo>,
}

/// A container for information about all MLB games on a specific date.
#[derive(Debug, Deserialize, Serialize)]
struct MlbGameDateInfo {
    games: Vec<MlbGameInfo>,
}

/// A container for information about an MLB game.
#[derive(Debug, Deserialize, Serialize)]
struct MlbGameInfo {
    gameDate: String,
    teams: MlbGameTeams,
    content: MlbGameContent,
}

/// A container for information about the two teams involved in an MLB game.
#[derive(Debug, Deserialize, Serialize)]
struct MlbGameTeams {
    away: MlbGameTeamInfo,
    home: MlbGameTeamInfo,
}

/// A container for information about an MLB team involved in a game.
#[derive(Debug, Deserialize, Serialize)]
struct MlbGameTeamInfo {
    team: MlbTeamInfo,
}

/// A container for static information about an MLB team.
#[derive(Debug, Deserialize, Serialize)]
struct MlbTeamInfo {
    name: String,
}

/// A container for information about media pertaining to an MLB game.
#[derive(Debug, Deserialize, Serialize)]
struct MlbGameContent {
    editorial: Option<MlbGameEditorial>,
}

/// A container for information about media pertaining to an MLB game.
#[derive(Debug, Deserialize, Serialize)]
struct MlbGameEditorial {
    recap: MlbGameRecap,
}

/// A container for information about media pertaining to an MLB game.
#[derive(Debug, Deserialize, Serialize)]
struct MlbGameRecap {
    mlb: Option<MlbGameArticle>,
}

/// A container for information about media pertaining to an MLB game.
#[derive(Debug, Deserialize, Serialize)]
pub struct MlbGameArticle {
    headline: String,
    image: MlbImageInfo,
}

/// A container for information about an image pertaining to an MLB game.
#[derive(Debug, Deserialize, Serialize)]
struct MlbImageInfo {
    cuts: Vec<MlbImageCuts>,
}

/// A container for information about an image pertaining to an MLB game.
#[derive(Debug, Deserialize, Serialize)]
struct MlbImageCuts {
    src: String,
}

/// A container for information used by the client to display an MLB game entry.
#[derive(Clone, Deserialize, Serialize)]
pub struct MlbGameClientInfo {
    pub title: String,
    pub image: Option<Vec<u8>>,
    pub summary: String,
}

/// Retrieves information about all games over a period of time.
pub async fn get_games() -> BTreeMap<NaiveDate, Vec<MlbGameClientInfo>> {
    let today = Local::now();
    let timezone = today.timezone();
    let client = Client::new();

    let offsets: Vec<i64> = vec![0, -1, -2];
    let mut futures = Vec::with_capacity(offsets.len());
    let mut results = BTreeMap::new();
    for i in offsets {
        let day = today + Duration::days(i);
        futures.push(extract_day_info(day, &timezone, &client));
    }

    for future in futures::future::join_all(futures).await {
        match future {
            Ok((day, info)) => {
                results.insert(day, info);
            }
            Err(ex) => error!("Error while retrieving game data: \n{}", ex),
        }
    }
    results
}

/// Retrieves information about all games for the given day.
///
/// # Errors
/// * If the URL is malformed.
/// * If the URL cannot be reached.
/// * If data cannot be read from the GET response.
/// * If the data cannot be deserialized into the expected JSON object.
async fn extract_day_info<Tz>(
    day: DateTime<Local>,
    timezone: &Tz,
    client: &Client<HttpConnector, Body>,
) -> Result<(NaiveDate, Vec<MlbGameClientInfo>), Box<dyn std::error::Error>>
where
    Tz: TimeZone,
    Tz::Offset: Display,
{
    let day_api = GAME_API.to_owned() + &format!("{}", day.format("%Y-%m-%d"));
    let day_uri = day_api.parse::<hyper::Uri>()?;
    let get_result = client.get(day_uri).await?;
    let text_buf = hyper::body::to_bytes(get_result).await?;
    let day_text = String::from_utf8(text_buf.as_ref().to_vec())?;
    let day_result = serde_json::from_str::<MlbGameRange>(&day_text)?;

    Ok((
        day.date().naive_local(),
        crate::extract_game_info(day_result, timezone).await,
    ))
}

/// Extracts the information for each game in the given MlbGameRange.
async fn extract_game_info<Tz>(day_results: MlbGameRange, timezone: &Tz) -> Vec<MlbGameClientInfo>
where
    Tz: TimeZone,
    Tz::Offset: Display,
{
    if let Some(game_day) = day_results.dates.get(0) {
        let client = Client::builder().build::<_, Body>(HttpsConnector::new());
        let num_games = game_day.games.len();
        let mut futures = Vec::with_capacity(num_games);
        let mut results = Vec::with_capacity(num_games);
        for game in &day_results.dates[0].games {
            futures.push(crate::extract_client_info(game, timezone, &client));
        }

        for info in futures::future::join_all(futures).await {
            results.push(info);
        }
        results
    } else {
        vec![]
    }
}

/// Extracts the client display information from the given game info.
async fn extract_client_info<Tz>(
    game: &MlbGameInfo,
    timezone: &Tz,
    client: &Client<HttpsConnector<HttpConnector>, Body>,
) -> MlbGameClientInfo
where
    Tz: TimeZone,
    Tz::Offset: Display,
{
    let teams = &game.teams;
    let title = format!("{} at {}", teams.away.team.name, teams.home.team.name);
    let time = game
        .gameDate
        .parse::<DateTime<Utc>>()
        .expect("Unable to parse time")
        .with_timezone(timezone);
    let default_summary = format!("Live {}", time.format("%I:%M %p"));
    let (image, summary) = if let Some(editorial) = &game.content.editorial {
        if let Some(article) = &editorial.recap.mlb {
            match extract_image(&article.image.cuts[0].src, client).await {
                Ok(img_bytes) => (Some(img_bytes), article.headline.to_owned()),
                Err(ex) => {
                    error!("Error while retrieving image for {}: \n{}", title, ex);
                    (None, default_summary)
                }
            }
        } else {
            (None, default_summary)
        }
    } else {
        (None, default_summary)
    };
    MlbGameClientInfo { title, image, summary }
}

/// Extracts the raw bytes of an image at the given URL.
///
/// # Errors
/// * If the URL is malformed.
/// * If the URL cannot be reached.
/// * If data cannot be read from the GET response.
///
async fn extract_image(
    img_url: &str,
    client: &Client<HttpsConnector<HttpConnector>, Body>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let img_uri = img_url.parse::<hyper::Uri>()?;
    let get_result = client.get(img_uri).await?;
    let img_bytes = hyper::body::to_bytes(get_result).await?;
    Ok(img_bytes.as_ref().to_vec())
}
