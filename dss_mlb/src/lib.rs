#![allow(non_snake_case)]

use chrono::{DateTime, Duration, Local, TimeZone, Utc};
use serde_derive::{Deserialize, Serialize};
use std::fmt::Display;

const GAME_API: &str =
    "http://statsapi.mlb.com/api/v1/schedule?hydrate=game(content(editorial(recap))),decisions&sportId=1&date=";

/// A container for MLB game information over a range of dates.
#[derive(Debug, Deserialize, Serialize)]
pub struct MlbGameRange {
    totalGames: u32,
    dates: Vec<MlbGameDateInfo>,
}

/// A container for MLB game information on a specific date.
#[derive(Debug, Deserialize, Serialize)]
pub struct MlbGameDateInfo {
    date: String,
    totalGames: u32,
    games: Vec<MlbGameInfo>,
}

/// A container for information about an MLB game.
#[derive(Debug, Deserialize, Serialize)]
pub struct MlbGameInfo {
    gamePk: u32,
    link: String,
    gameType: String,
    season: String,
    gameDate: String,
    officialDate: String,
    status: MlbGameStatus,
    teams: MlbGameTeams,
    decisions: Option<MlbGameDecisions>,
    venue: MlbVenueInfo,
    content: MlbGameContent,
    isTie: Option<bool>,
    gameNumber: u32,
    publicFacing: bool,
    doubleHeader: String,
    gamedayType: String,
    tiebreaker: String,
    calendarEventID: String,
    seasonDisplay: String,
    dayNight: String,
    scheduledInnings: u32,
    inningBreakLength: u32,
    gamesInSeries: u32,
    seriesGameNumber: u32,
    seriesDescription: String,
    recordSource: String,
    ifNecessary: String,
    ifNecessaryDescription: String,
}

/// A container for the status of an MLB game.
#[derive(Debug, Deserialize, Serialize)]
pub struct MlbGameStatus {
    abstractGameState: String,
    codedGameState: String,
    detailedState: String,
    statusCode: String,
    reason: Option<String>,
    abstractGameCode: String,
}

/// A container for the two teams involved in an MLB game.
#[derive(Debug, Deserialize, Serialize)]
pub struct MlbGameTeams {
    away: MlbGameTeamInfo,
    home: MlbGameTeamInfo,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MlbGameTeamInfo {
    leagueRecord: MlbTeamRecord,
    score: Option<u32>,
    team: MlbTeamInfo,
    isWinner: Option<bool>,
    splitSquad: bool,
    seriesNumber: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MlbGameDecisions {
    winner: MlbPlayerInfo,
    loser: MlbPlayerInfo,
    save: Option<MlbPlayerInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MlbPlayerInfo {
    id: u32,
    fullName: String,
    link: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MlbTeamInfo {
    id: u32,
    name: String,
    link: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MlbTeamRecord {
    wins: u32,
    losses: u32,
    pct: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MlbVenueInfo {
    id: u32,
    name: String,
    link: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MlbGameContent {
    link: String,
    editorial: Option<MlbGameEditorial>,
    media: Option<MlbGameMedia>,
    highlights: Option<MlbPlaceholderInfo>,
    summary: Option<MlbPlaceholderInfo>,
    gameNotes: Option<MlbPlaceholderInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MlbPlaceholderInfo {}

#[derive(Debug, Deserialize, Serialize)]
pub struct MlbGameEditorial {
    recap: MlbGameRecap,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MlbGameRecap {
    mlb: Option<MlbGameArticle>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MlbGameArticle {
    r#type: String,
    state: String,
    date: String,
    headline: String,
    seoTitle: String,
    slug: String,
    blurb: String,
    keywordsAll: Vec<MlbKeywordInfo>,
    keywordsDisplay: Vec<String>,
    image: MlbImageInfo,
    subhead: Option<String>,
    seoKeywords: String,
    body: String,
    contributors: Vec<MlbContributorInfo>,
    photo: MlbImageInfo,
    url: String,
    media: Option<MlbGameArticleMedia>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MlbGameArticleMedia {
    r#type: String,
    state: String,
    date: String,
    id: String,
    headline: String,
    seoTitle: String,
    slug: String,
    blurb: String,
    keywordsAll: Vec<MlbKeywordInfo>,
    keywordsDisplay: Vec<String>,
    image: MlbImageInfo,
    noIndex: bool,
    mediaPlaybackId: String,
    title: String,
    description: String,
    duration: String,
    guid: Option<String>,
    mediaPlaybackUrl: String,
    playbacks: Vec<MlbGameMediaPlaybackInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MlbGameMediaPlaybackInfo {
    name: String,
    url: String,
    width: String,
    height: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MlbGameMedia {
    freeGame: bool,
    enhancedGame: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MlbContributorInfo {
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MlbKeywordInfo {
    r#type: Option<String>,
    value: String,
    displayName: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MlbImageInfo {
    title: String,
    altText: Option<String>,
    cuts: Vec<MlbImageCuts>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MlbImageCuts {
    aspectRatio: String,
    width: u32,
    height: u32,
    src: String,
    at2x: String,
    at3x: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct MlbGameClientInfo {
    pub title: String,
    pub image: Option<Vec<u8>>,
    pub summary: String,
}

pub struct MlbManager {}

impl MlbManager {
    pub async fn get_games(&self) -> Result<Vec<Vec<MlbGameClientInfo>>, reqwest::Error> {
        let today = Local::now();
        let timezone = today.timezone();

        let offsets: Vec<i64> = vec![-2, -1, 0];
        let mut result = Vec::with_capacity(offsets.len());
        for i in offsets {
            let day = today + Duration::days(i);
            let day_api = GAME_API.to_owned() + &format!("{}", day.format("%Y-%m-%d"));
            let day_text = reqwest::get(&day_api).await?.text().await?;
            let day_result = serde_json::from_str::<MlbGameRange>(&day_text).unwrap();
            result.push(crate::extract_client_info(day_result, &timezone).await?);
            println!("parsed {}", i);
        }

        Ok(result)
    }
}

async fn extract_client_info<Tz>(
    day_results: MlbGameRange,
    timezone: &Tz,
) -> Result<Vec<MlbGameClientInfo>, reqwest::Error>
where
    Tz: TimeZone,
    Tz::Offset: Display,
{
    if let Some(game_day) = day_results.dates.get(0) {
        let num_games = game_day.games.len();
        println!("parsing {} games", num_games);
        let mut result = Vec::with_capacity(num_games);
        for game in &day_results.dates[0].games {
            let teams = &game.teams;
            let title = format!("{} at {}", teams.away.team.name, teams.home.team.name);
            let time = game
                .gameDate
                .parse::<DateTime<Utc>>()
                .expect("Unable to parse time")
                .with_timezone(timezone);
            let default_summary = format!("Live {}", time);
            let (image, summary) = if let Some(editorial) = &game.content.editorial {
                if let Some(article) = &editorial.recap.mlb {
                    println!("loading image from {}", &article.image.cuts[0].src);
                    let img_bytes = reqwest::get(&article.image.cuts[0].src).await?.bytes().await?;
                    
                    (Some(img_bytes.as_ref().to_vec()), article.headline.to_owned())
                } else {
                    (None, default_summary)
                }
            } else {
                (None, default_summary)
            };
            result.push(MlbGameClientInfo { title, image, summary });
        }
        Ok(result)
    } else {
        Ok(vec![])
    }
}
