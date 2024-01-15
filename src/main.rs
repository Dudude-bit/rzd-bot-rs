mod rzd;
mod db;

use std::env;
use std::path::Path;
use std::sync::Arc;

use chrono::NaiveDate;
use env_logger;
use log::LevelFilter;
use teloxide::types::InputFile;
use teloxide::{
    dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler},
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
    utils::command::BotCommands,
};
use speedb::{DB, Options};
use crate::db::RZDDb;
use crate::rzd::RZDApi;

const CUPE_TYPE: &str = "купе";

type RZDDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    Start,
    Cancel,
    Niggers,
    Dimok,
    Ss,
}

#[derive(Debug, Clone)]
pub struct Train {
    code0: String,
    code1: String,
    dt0: String,
    time0: String,
    tnum0: String,
}

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    ReceiveFromPoint,
    ChooseFromPointCode,
    ReceiveToPoint {
        from_point_code: String,
    },
    ChooseToPointCode {
        from_point_code: String,
    },
    ReceiveDate {
        from_point_code: String,
        to_point_code: String,
    },
    ChooseTrain {
        trains: Vec<Train>,
    },
}

#[tokio::main]
async fn main() {
    env_logger::builder().filter_level(LevelFilter::Info).init();
    let mut db_path = env::var("DB_PATH").unwrap_or_default();
    if db_path.is_empty() {
        log::warn!("DB_PATH is empty. Creating default file db.db");
        db_path = "db.db".to_string();
    }

    if !Path::exists(db_path.clone().as_ref()) {
        log::warn!("DB_PATH {db_path} does not exists, creating");
    }

    let mut options = Options::default();
    options.create_if_missing(true);
    let db = DB::open(&options, db_path).expect("cant create db");

    let rzd_db = RZDDb::new(db);

    let bot = Bot::from_env();

    log::info!("bot is starting");

    let rzd_api = rzd::RZDApi::new();
    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![InMemStorage::<State>::new(), rzd_api, rzd_db])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![Command::Start].endpoint(start))
        .branch(case![Command::Cancel].endpoint(cancel))
        .branch(case![Command::Niggers].endpoint(niggers))
        .branch(case![Command::Dimok].endpoint(dimok))
        .branch(case![Command::Ss].endpoint(ss));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![State::ReceiveFromPoint].endpoint(receive_from_point))
        .branch(case![State::ReceiveToPoint { from_point_code }].endpoint(receive_to_point))
        .branch(
            case![State::ReceiveDate {
                from_point_code,
                to_point_code,
            }]
            .endpoint(receive_date),
        )
        .branch(case![State::ChooseTrain { trains }].endpoint(receive_train_idx));

    let callback_query_handler = Update::filter_callback_query()
        .branch(case![State::ChooseFromPointCode].endpoint(choose_from_point_code))
        .branch(case![State::ChooseToPointCode { from_point_code }].endpoint(choose_to_point_code))
        .branch(case![State::ChooseTrain { trains }].endpoint(poll_day));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler)
        .branch(callback_query_handler)
}

async fn start(bot: Bot, dialogue: RZDDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Write from point").await?;
    dialogue.update(State::ReceiveFromPoint).await?;
    Ok(())
}

async fn cancel(bot: Bot, dialogue: RZDDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "You canceled it").await?;
    dialogue.reset().await?;
    Ok(())
}

async fn receive_from_point(bot: Bot, dialogue: RZDDialogue, rzd_api: Arc<RZDApi>, msg: Message) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            let codes = rzd_api.get_rzd_point_codes(text.into(), 5).await;
            match codes {
                Ok(codes) => {
                    let mut reply_markup = Vec::new();
                    for code in codes.iter().clone() {
                        reply_markup.push(InlineKeyboardButton::callback(
                            code.name.clone(),
                            code.code.clone(),
                        ))
                    }
                    bot.send_message(msg.chat.id, "Choose from point")
                        .reply_markup(InlineKeyboardMarkup::new([reply_markup]))
                        .await?;
                    dialogue.update(State::ChooseFromPointCode {}).await?;
                }
                Err(err) => {
                    bot.send_message(
                        msg.chat.id,
                        format!("Error on getting rzd point codes {}", err),
                    )
                    .await?;
                }
            }
        }
        None => {
            bot.send_message(msg.chat.id, "Send me plain text.").await?;
        }
    }

    Ok(())
}

async fn choose_from_point_code(
    bot: Bot,
    dialogue: RZDDialogue,
    q: CallbackQuery,
) -> HandlerResult {
    bot.answer_callback_query(q.id).await?;
    if let Some(code) = &q.data {
        bot.send_message(dialogue.chat_id(), "Write to point")
            .await?;
        dialogue
            .update(State::ReceiveToPoint {
                from_point_code: code.into(),
            })
            .await?;
    }
    Ok(())
}

async fn receive_to_point(
    bot: Bot,
    dialogue: RZDDialogue,
    rzd_api: Arc<RZDApi>,
    from_point_code: String,
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            let codes = rzd_api.get_rzd_point_codes(text.into(), 5).await;
            match codes {
                Ok(codes) => {
                    let mut reply_markup = Vec::new();
                    for code in codes.iter().clone() {
                        reply_markup.push(InlineKeyboardButton::callback(
                            code.name.clone(),
                            code.code.clone(),
                        ))
                    }
                    bot.send_message(msg.chat.id, "Choose to point")
                        .reply_markup(InlineKeyboardMarkup::new([reply_markup]))
                        .await?;
                    dialogue
                        .update(State::ChooseToPointCode { from_point_code })
                        .await?;
                }
                Err(err) => {
                    bot.send_message(
                        msg.chat.id,
                        format!("Error on getting rzd point codes {}", err),
                    )
                    .await?;
                }
            }
        }
        None => {
            bot.send_message(msg.chat.id, "Send me plain text.").await?;
        }
    }

    Ok(())
}

async fn choose_to_point_code(
    bot: Bot,
    dialogue: RZDDialogue,
    from_point_code: String,
    q: CallbackQuery,
) -> HandlerResult {
    bot.answer_callback_query(q.id).await?;
    if let Some(code) = &q.data {
        bot.send_message(dialogue.chat_id(), "Write a date in format(d.m.y)")
            .await?;
        dialogue
            .update(State::ReceiveDate {
                from_point_code,
                to_point_code: code.into(),
            })
            .await?;
    }
    Ok(())
}

async fn receive_date(
    bot: Bot,
    dialogue: RZDDialogue,
    rzd_api: Arc<RZDApi>,
    (from_point_code, to_point_code): (String, String),
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(date) => {
            let date = NaiveDate::parse_from_str(date, "%d.%m.%Y");
            match date {
                Ok(date) => {
                    // TODO check if date not less than now
                    let trains = rzd_api.get_trains_from_rzd(
                        from_point_code.clone(),
                        to_point_code.clone(),
                        date.format("%d.%m.%Y").to_string(),
                        5,
                    )
                    .await;
                    match trains {
                        Ok(trains) => {
                            let mut trains_state: Vec<Train> = Vec::new();
                            let mut message_text: String = String::new();
                            let mut idx_counter = 1;
                            for train in trains.tp[0].list.iter() {
                                let mut cupe_count_type = 0;
                                for car in train.cars.iter() {
                                    if car._type.to_lowercase() == CUPE_TYPE && !car.disabled_person
                                    {
                                        cupe_count_type += car.free_seats
                                    }
                                }
                                if cupe_count_type == 0 {
                                    continue;
                                }
                                trains_state.push(Train {
                                    code0: from_point_code.clone(),
                                    code1: to_point_code.clone(),
                                    dt0: train.date0.clone(),
                                    time0: train.time0.clone(),
                                    tnum0: train.number.clone(),
                                });
                                message_text.push_str(format!("{0}. Поезд: {1}\nДата отбытия: {2} \nВремя отбытия: {3}\nКоличество свободных мест в купе: {cupe_count_type}\n", idx_counter, train.number, train.date0, train.time0).as_str());
                                idx_counter += 1;
                            }
                            if message_text.is_empty() {
                                bot.send_message(msg.chat.id, "Not found. Please type /start to try again. Current dialogue reseted").await?;
                                dialogue.reset().await?;
                            } else {
                                let mut reply_markup = Vec::new();
                                reply_markup.push(InlineKeyboardButton::callback(
                                    "Poll this day",
                                    format!(
                                        "{from_point_code}_{to_point_code}_{}",
                                        date.format("%d.%m.%Y").to_string()
                                    ),
                                ));
                                bot.send_message(msg.chat.id, message_text)
                                    .reply_markup(InlineKeyboardMarkup::new([reply_markup]))
                                    .await?;
                                dialogue
                                    .update(State::ChooseTrain {
                                        trains: trains_state,
                                    })
                                    .await?;
                            }
                        }
                        Err(err) => {
                            bot.send_message(
                                msg.chat.id,
                                format!(
                                    "Error on getting rzd trains {err}. Current dialogue canceled"
                                ),
                            )
                            .await?;
                            dialogue.reset().await?;
                        }
                    }
                }
                Err(err) => {
                    bot.send_message(
                        msg.chat.id,
                        format!("Error on parsing date {}. Current dialogue canceled", err),
                    )
                    .await?;
                    dialogue.reset().await?;
                }
            }
        }
        None => {
            bot.send_message(msg.chat.id, "Send me plain text.").await?;
        }
    }

    Ok(())
}

async fn receive_train_idx(
    bot: Bot,
    dialogue: RZDDialogue,
    rzd_api: Arc<RZDApi>,
    trains: Vec<Train>,
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(idx) => {
            let idx: usize = idx.parse().unwrap_or(0);
            if idx == 0 {
                bot.send_message(msg.chat.id, "Negative index. Current dialogue canceled")
                    .await?;
                dialogue.reset().await?;
                return Ok(());
            }
            let train = trains.get(idx - 1);
            if train.is_none() {
                bot.send_message(msg.chat.id, "Invalid index. Current dialogue canceled")
                    .await?;
                dialogue.reset().await?;
                return Ok(());
            }
            let train = train.unwrap();
            let carriages = rzd_api.get_trains_carriages_from_rzd(
                train.code0.clone(),
                train.code1.clone(),
                train.dt0.clone(),
                train.time0.clone(),
                train.tnum0.clone(),
                5,
            )
            .await;
            match carriages {
                Ok(v) => {
                    let mut message_text: String = String::new();
                    for car in v.lst[0].cars.iter() {
                        if car._type.to_lowercase() != CUPE_TYPE {
                            continue;
                        }
                        for place in car.places.iter() {
                            let places_ref = place.split('-').collect::<Vec<&str>>();
                            if places_ref.len() != 2 {
                                continue;
                            }
                            let (start_place, end_place) = (
                                places_ref[0]
                                    .trim_end_matches(|c: char| c.is_alphabetic())
                                    .parse::<isize>()
                                    .unwrap(),
                                places_ref[1]
                                    .trim_end_matches(|c: char| c.is_alphabetic())
                                    .parse::<isize>()
                                    .unwrap(),
                            );
                            if start_place > end_place {
                                log::warn!("start_place {} is greater than end_place {} with params code0 = {}, code1 = {}, dt0 = {}, time0 = {}, tnum0 = {}", start_place, end_place, train.code0, train.code1, train.dt0, train.time0, train.tnum0);
                                continue;
                            }
                            for place_n in start_place..=end_place {
                                //
                                if place_n % 4 == 1 && end_place - place_n >= 3 {
                                    // Blyat ya ne vspomnu cherez god logiku
                                    message_text.push_str(
                                        format!(
                                            "Номер вагона: {}\nНомер мест: {} - {}\n",
                                            car.cnumber,
                                            place_n,
                                            place_n + 3
                                        )
                                        .as_str(),
                                    )
                                }
                            }
                        }
                    }
                    if message_text.is_empty() {
                        bot.send_message(
                            msg.chat.id,
                            "Not found. Please type /start to try again. Current dialogue reseted",
                        )
                        .await?;
                        dialogue.reset().await?;
                    } else {
                        bot.send_message(msg.chat.id, message_text + "Current dialogue reseted")
                            .await?;
                        dialogue.reset().await?;
                    }
                }
                Err(err) => {
                    bot.send_message(
                        msg.chat.id,
                        format!(
                            "Error on getting rzd train carriages {err}. Current dialogue canceled"
                        ),
                    )
                    .await?;
                    dialogue.reset().await?;
                }
            }
        }
        None => {
            bot.send_message(msg.chat.id, "Send me plain text.").await?;
        }
    }
    Ok(())
}

async fn poll_day(bot: Bot, dialogue: RZDDialogue,rzd_db: Arc<RZDDb>, q: CallbackQuery) -> HandlerResult {
    bot.answer_callback_query(q.id).await?;
    if let Some(data) = &q.data {
        let splitted_data = data.split('_').collect::<Vec<&str>>();
        println!("{:?}", splitted_data);
        if splitted_data.len() != 3 {
            bot.send_message(dialogue.chat_id(), "Invalid length of callback data")
                .await?;
        }
    }
    Ok(())
}

async fn poll_train(bot: Bot, dialogue: RZDDialogue,rzd_db: Arc<RZDDb>, q: CallbackQuery) -> HandlerResult {
    bot.answer_callback_query(q.id).await?;
    Ok(())
}

async fn niggers(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Негры пидорасы").await?;
    Ok(())
}

async fn dimok(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, include_str!("static/dimok.txt"))
        .await?;
    bot.send_audio(
        msg.chat.id,
        InputFile::memory(include_bytes!("static/дымок.mp3").as_slice()).file_name("Дымок"),
    )
    .await?;
    Ok(())
}

async fn ss(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, include_str!("static/гимн_люфтваффе.txt"))
        .await?;

    bot.send_audio(
        msg.chat.id,
        InputFile::memory(include_bytes!("static/гимн_люфтваффе.mp3").as_slice())
            .file_name("Гимн люфтваффе"),
    )
    .await?;
    Ok(())
}
