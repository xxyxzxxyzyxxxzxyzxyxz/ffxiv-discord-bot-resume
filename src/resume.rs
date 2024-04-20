use crate::achievement_list::{
    ACHIEVEMENTS_BLUEMAGE, ACHIEVEMENTS_PUBLIC, ACHIEVEMENTS_DEEP,
    ACHIEVEMENTS_ANOTHER, ACHIEVEMENTS_SAVAGE, ACHIEVEMENTS_ULTIMATE,
};

use reqwest;
use scraper::{Html, Selector};
use std::collections::HashMap;
use regex::Regex;
use chrono::DateTime;

struct AchievementData {
    index: i64,
    title: String,
    days_to_achieve: i64,
    flag_strict: String,
    flag_lenient: String,
}

pub async fn get_resume(
    character_id: &str,
    resume_type: &str
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let url = format!(
            "https://jp.finalfantasyxiv.com/lodestone/character/{}/achievement/kind/1/#anchor_achievement",
            character_id
        );
    let response = reqwest::get(&url).await.unwrap();
    let html = response.text().await.unwrap();

    let (achievement_map_actual, character_name, home_world) = {
        let entry_selector = Selector::parse(r#"li.entry"#).unwrap();
        let text_selector = Selector::parse(r#"p.entry__activity__txt"#).unwrap();
        let time_selector = Selector::parse(r#"time.entry__activity__time script"#).unwrap();
        let regex = Regex::new(r#"ldst_strftime\((\d+),"#).unwrap();
        let character_name_selector = Selector::parse(r#"p.frame__chara__name"#).unwrap();
        let home_world_selector = Selector::parse(r#"p.frame__chara__world"#).unwrap();
        let home_world_regex = Regex::new(r#"<i[^>]*></i>([^<]+)"#).unwrap();
        let document = Html::parse_document(&html);

        let character_name = document
            .select(&character_name_selector)
            .next()
            .map(|n| n.text().collect::<Vec<_>>().join(""))
            .unwrap_or_default();

        let home_world = document
            .select(&home_world_selector)
            .next()
            .map(|w| {
                let world_html = w.html();
                home_world_regex
                    .captures(&world_html)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_default()
            })
            .unwrap_or_default();
    
        let mut achievement_map_actual = HashMap::new();
        for entry in document.select(&entry_selector) {
            if let Some(text) = entry
                .select(&text_selector)
                .next()
                .map(|t| t.text().collect::<Vec<_>>().join(""))
            {
                if let Some(script) = entry.select(&time_selector).next() {
                    if let Some(captures) = regex.captures(&script.html()) {
                        let arg = captures[1].parse::<i64>().unwrap();
                        achievement_map_actual.insert(text, arg);
                    }
                }
            }
        }

        (achievement_map_actual, character_name, home_world)
    };

    let achievements_list;
    if resume_type == "all" {
        let mut all_achievements = Vec::new();
        all_achievements.extend_from_slice(ACHIEVEMENTS_ULTIMATE);
        all_achievements.extend_from_slice(ACHIEVEMENTS_SAVAGE);
        all_achievements.extend_from_slice(ACHIEVEMENTS_BLUEMAGE);
        all_achievements.extend_from_slice(ACHIEVEMENTS_ANOTHER);
        all_achievements.extend_from_slice(ACHIEVEMENTS_PUBLIC);
        all_achievements.extend_from_slice(ACHIEVEMENTS_DEEP);
        achievements_list = all_achievements;
    } else {
        achievements_list = match resume_type {
            "u" => ACHIEVEMENTS_ULTIMATE.to_vec(),
            "s" => ACHIEVEMENTS_SAVAGE.to_vec(),
            "bm" => ACHIEVEMENTS_BLUEMAGE.to_vec(),
            "ad" => ACHIEVEMENTS_ANOTHER.to_vec(),
            "pd" => ACHIEVEMENTS_PUBLIC.to_vec(),
            "dd" => ACHIEVEMENTS_DEEP.to_vec(),
            _ => Vec::new(),
        };
    }

    let achievement_map: HashMap<_, _> = achievements_list
        .iter()
        .map(|(k, (v1, v2, v3, v4, v5))| (k.to_string(), (*v1, *v2, *v3, *v4, *v5)))
        .collect();

    let mut achievement_data = Vec::new();
    for (achievement, (index, title, utime_start, utime_end_strict, utime_end_lenient)) in &achievement_map {
        if let Some(utime_achieve) = achievement_map_actual.get(achievement) {
            let datetime_achieve = DateTime::from_timestamp(*utime_achieve, 0).unwrap();
            let datetime_start = DateTime::from_timestamp(*utime_start, 0).unwrap();
            let datetime_end_strict = DateTime::from_timestamp(*utime_end_strict, 0).unwrap();
            let datetime_end_lenient = DateTime::from_timestamp(*utime_end_lenient, 0).unwrap();
            let days_to_achieve = (datetime_achieve - datetime_start).num_days();
            let days_to_strict = (datetime_end_strict - datetime_start).num_days();
            let days_to_lenient = (datetime_end_lenient - datetime_start).num_days();
            let flag_strict = if days_to_achieve <= days_to_strict { '○' } else { '×' };
            let flag_lenient = if days_to_achieve <= days_to_lenient { '○' } else { '×' };
            achievement_data.push(
                AchievementData{
                    index: *index,
                    title: title.to_string(),
                    days_to_achieve,
                    flag_strict: flag_strict.to_string(),
                    flag_lenient: flag_lenient.to_string(),
                }
            );
        }
    }

    achievement_data.sort_by_key(|data| data.index);

    let  character_information = format!("{} @ {}\n", &character_name, &home_world);
    let mut character_resume = String::new();
    for data in achievement_data {
        let line = format!(
            "{}: {} days (in minor patch: {}/in major patch: {})\n",
            data.title,
            data.days_to_achieve,
            data.flag_strict,
            data.flag_lenient
        );

        character_resume.push_str(&line);
    }

    Ok((character_information, character_resume))
}