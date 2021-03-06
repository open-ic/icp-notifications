mod dynamodb;
mod ses;

#[allow(dead_code)]
mod sns;

use crate::dynamodb::DynamoDbClient;
use crate::ses::SesClient;
use crate::sns::SnsClient;
use candid::Principal;
use lambda_runtime::{handler_fn, Context};
use log::info;
use notification_sender::actions::{remove_notifications, send_notifications};
use notification_sender::ic_agent::{IcAgent, IcAgentConfig};
use serde::Deserialize;
use std::str::FromStr;
use types::Error;

#[derive(Deserialize)]
struct Request {
    run_mode: Mode,
}

#[derive(Deserialize)]
enum Mode {
    SendNotifications,
    RemoveNotifications,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv()?;
    env_logger::init();
    info!("Starting...");
    let func = handler_fn(handler);
    lambda_runtime::run(func).await?;
    Ok(())
}

async fn handler(request: Request, _: Context) -> Result<(), Error> {
    let canister_id = Principal::from_text(dotenv::var("NOTIFICATIONS_CANISTER_ID")?).unwrap();
    let dynamodb_client = DynamoDbClient::build(canister_id);
    let ic_url = dotenv::var("IC_URL")?;
    let ic_identity_pem = dotenv::var("IC_IDENTITY_PEM")?;
    let is_development = bool::from_str(&dotenv::var("IS_DEVELOPMENT")?).unwrap();

    let ic_agent_config = IcAgentConfig {
        ic_url,
        ic_identity_pem,
        fetch_root_key: is_development,
    };
    let ic_agent = IcAgent::build(&ic_agent_config, canister_id).await?;

    info!("Configuration complete");

    match request.run_mode {
        Mode::SendNotifications => {
            let sns_client = SnsClient::build()?;
            let ses_client = SesClient::build()?;
            send_notifications::run(&ic_agent, &dynamodb_client, &sns_client, &ses_client).await
        }
        Mode::RemoveNotifications => remove_notifications::run(&ic_agent, &dynamodb_client).await,
    }
}
