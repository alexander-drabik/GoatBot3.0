use std::sync::Arc;

use ::serenity::all::{CreateAllowedMentions, Mentionable};
use chrono::{Duration, Utc};
use poise::{CreateReply, Prefix, PrefixFrameworkOptions, serenity_prelude as serenity};
use sqlx::sqlite::SqlitePool;
use stats::Stats;
use tokio::fs;
use user::ServerUser;

mod stats;
mod user;

struct Data {
    pool: Arc<SqlitePool>,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[poise::command(slash_command, prefix_command)]
async fn info(
    ctx: Context<'_>,
    #[description = "Wybrany użytkownik"] dc_user: Option<serenity::User>,
) -> Result<(), Error> {
    println!("{}", ctx.author().id.get());
    if let Some(dc_user) = dc_user {
        let server_user =
            ServerUser::get_user_from_id(&ctx.data().pool, dc_user.id.get() as i64).await;
        let response = if let Some(server_user) = server_user {
            format!(
                "**Użytkownik**: {} \n\
                - **Liczba Wiadomości**: {} \n\
                - **Pozostałe Wyciszenia**: {}\n\
                - **Wyciszeni Użytkownicy**: {}\n\
                - **Ostatnia Aktywność**: {}\n\
                - **Passa**: {} {}",
                dc_user.mention(),
                server_user.message_count,
                server_user.mutes_left,
                server_user.mutes_used,
                server_user.last_activity,
                server_user.streak,
                if server_user.streak == 1 {
                    "dzień"
                } else {
                    "dni"
                }
            )
        } else {
            String::from("Nie znaleziono użytkownika!")
        };
        ctx.send(CreateReply {
            content: Some(response),
            allowed_mentions: Some(CreateAllowedMentions::new().empty_users()),
            ..Default::default()
        })
        .await?;
    }

    Ok(())
}

async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            println!("Logged in as {}", data_about_bot.user.name);
        }
        serenity::FullEvent::Message { new_message } => {
            if new_message.author.bot {
                return Ok(());
            }

            let author_id = new_message.author.id.get() as i64;
            ServerUser::increment_message_count(&data.pool, author_id)
                .await
                .expect("Error incrementing message count!");
            Stats::increment_message_count(&data.pool, Utc::now().date_naive())
                .await
                .expect("Error incrementing message count in stats!");

            if let Some(mut s_user) = ServerUser::get_user_from_id(&data.pool, author_id).await {
                let current_date = Utc::now().date_naive();
                let mut need_to_update = false;

                let duration = current_date.signed_duration_since(s_user.last_activity);
                if duration == Duration::days(1) {
                    s_user.streak += 1;
                    need_to_update = true;
                } else if s_user.last_activity != current_date {
                    s_user.last_activity = current_date;
                    s_user.streak = 0;
                    need_to_update = true;
                }

                if s_user.message_count % 100 == 0 {
                    s_user.mutes_left += 1;
                    need_to_update = true;
                }

                if need_to_update {
                    s_user.update_db(&data.pool).await;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let database_url = "sqlite://./.data/users.sqlite";
    let pool = Arc::new(
        SqlitePool::connect(database_url)
            .await
            .expect("Cannot connect to the db"),
    );

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY,
            message_count INTEGER,
            mutes_left INTEGER,
            mutes_used INTEGER,
            streak INTEGER,
            last_activity DATE
        )
        "#,
    )
    .execute(&*pool)
    .await
    .expect("Create table users query error");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS stats (
            date DATE PRIMARY KEY,
            message_count INTEGER
        )
        "#,
    )
    .execute(&*pool)
    .await
    .expect("Create table stats query error");

    let token = fs::read_to_string(".token")
        .await
        .expect("Token not found!");
    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::MESSAGE_CONTENT
        | serenity::GatewayIntents::GUILD_MESSAGES;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![info()],
            prefix_options: PrefixFrameworkOptions {
                prefix: Some(String::from("!")),
                additional_prefixes: vec![Prefix::Literal("goatbot,")],
                ..Default::default()
            },
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    pool: Arc::clone(&pool),
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
