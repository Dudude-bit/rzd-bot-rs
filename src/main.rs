mod rzd;

use crate::rzd::{
    get_rzd_point_codes, get_trains_from_rzd,
};
use chrono::{NaiveDate, ParseResult};
use retry_macro::retry_async_sleep;
use std::fmt::format;
use teloxide::{
    dispatching::{dialogue, dialogue::InMemStorage, dialogue::RedisStorage, UpdateHandler},
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
    utils::command::BotCommands,
};

const CUPE_TYPE: &str = "купе";

type RZDDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    Start,
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
        from_point_code: String,
        to_point_code: String,
        date: String,
    },
}

#[tokio::main]
async fn main() {
    log::info!("Starting rzd bot");

    let bot = Bot::from_env();

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![State::Start].branch(case![Command::Start].endpoint(start)));

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
        );

    let callback_query_handler = Update::filter_callback_query()
        .branch(case![State::ChooseFromPointCode].endpoint(choose_from_point_code))
        .branch(case![State::ChooseToPointCode { from_point_code }].endpoint(choose_to_point_code));

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

async fn receive_from_point(bot: Bot, dialogue: RZDDialogue, msg: Message) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            let codes = get_rzd_point_codes(text.into()).await;
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
    from_point_code: String,
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            let codes = get_rzd_point_codes(text.into()).await;
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
                        .update(State::ChooseToPointCode {
                            from_point_code: from_point_code.into(),
                        })
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
    if let Some(code) = &q.data {
        bot.send_message(dialogue.chat_id(), "Write a date in format(d.m.y)")
            .await?;
        dialogue
            .update(State::ReceiveDate {
                from_point_code: from_point_code.into(),
                to_point_code: code.into(),
            })
            .await?;
    }
    Ok(())
}

async fn receive_date(
    bot: Bot,
    dialogue: RZDDialogue,
    (from_point_code, to_point_code): (String, String),
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(date) => {
            let date = NaiveDate::parse_from_str(date, "%d.%m.%Y");
            match date {
                Ok(date) => {
                    // Compare time if less -> error
                    let trains = get_trains_from_rzd(
                        from_point_code.clone(),
                        to_point_code.clone(),
                        date.format("%d.%m.%Y").to_string(),
                    )
                    .await;
                    match trains {
                        Ok(trains) => {
                            let mut message_text: String = String::new();
                            for (idx, train) in trains.tp[0].list.iter().enumerate() {
                                let mut cupe_count_type = 0;
                                for car in train.cars.iter() {
                                    if car._type == CUPE_TYPE && !car.disabled_person {
                                        cupe_count_type += car.free_seats
                                    }
                                }
                                if cupe_count_type == 0 {
                                    continue;
                                }
                                message_text.push_str(format!("{idx}. Поезд: {0}\nДата отбытия: {1} \nВремя отбытия: {2}\nКоличество свободных мест в купе: {cupe_count_type}\n", train.number, train.date0, train.time0).as_str());
                            }
                            if message_text.is_empty() {
                                bot.send_message(msg.chat.id, "Not found. Please type /start to try again. Current dialogue reseted").await?;
                                dialogue.reset().await?;
                            } else {
                                dialogue
                                    .update(State::ChooseTrain {
                                        from_point_code: from_point_code.into(),
                                        to_point_code: to_point_code.into(),
                                        date: date.format("%d.%m.%Y").to_string(),
                                    })
                                    .await?;
                            }
                        }
                        Err(err) => {
                            bot.send_message(
                                msg.chat.id,
                                format!("Error on getting rzd trains {err}"),
                            )
                            .await?;
                        }
                    }
                }
                Err(err) => {
                    bot.send_message(msg.chat.id, format!("Error on parsing date {}", err))
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
