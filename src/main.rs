#![deny(unused_must_use)]


use std::{
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    path::PathBuf,
    time::{Duration}
};
use twitch_types::{
    points::RewardId,
    UserId
};
use twitch_oauth2::{
    tokens::UserToken,
    types::AccessToken,
};
use twitch_api::{
    HelixClient,
    HttpClient,
    helix::points::{
        get_custom_reward::{
            GetCustomRewardRequest,
            CustomReward,
        },
        update_custom_reward::{
            UpdateCustomRewardBody,
            UpdateCustomRewardRequest,
        },
    },
};
use url::Url;

const DEBUG_MODE: bool = cfg!(debug_assertions);

mod flags;

type Res<A> = Result<A, Box<dyn std::error::Error>>;

const SLEEP_DURATION: std::time::Duration = std::time::Duration::from_millis(16);

pub type Token = String;

pub enum SpecKind {
    GetRewards,
    ModifyRewards(PathBuf)
}

pub enum TokenSpec {
    Token(Token),
    Auth(AuthSpec)
}

pub struct Spec {
    pub login_name: String,
    pub kind: SpecKind,
    pub token_spec: TokenSpec,
}

pub struct AuthSpec {
    addr: SocketAddr,
    /// The original string passed by the user
    addr_string: String,
    app_id: String,
    app_secret: String,
}

#[tokio::main]
pub async fn main() -> Res<()> {
    let args = flags::Args::from_env()?;

    let Spec {
        login_name,
        token_spec,
        kind,
    } = args.to_spec()?;

    tracing_subscriber::fmt::init();

    let access_token = match token_spec {
        TokenSpec::Auth(auth_spec) => authorize(auth_spec)?,
        TokenSpec::Token(token) => AccessToken::new(token),
    };

    tracing::info!("Debug mode: {DEBUG_MODE}");

    let agent: ureq::Agent = ureq::agent();

    let client: HelixClient<ureq::Agent> = HelixClient::with_client(agent);

    let user_token = UserToken::from_token(&client, access_token).await?;

    let calls = match kind {
        SpecKind::GetRewards => {
            vec![ApiCallSpec::GetRewards(GetRewardsSpec { 
                 //TODO? Allow specifying a different id here?
                broadcaster_id: user_token.user_id.clone(),
            })]
        },
        SpecKind::ModifyRewards(lua_path) => {
            use mlua::{Lua, Table}; 
            dbg!(&lua_path);

            let mut calls = Vec::with_capacity(50 /* max allowed apparently */);

            let mut lua_state = Lua::new();

            let expression: Table = lua_state.load(lua_path).eval()?;

            dbg!(expression);
            // TODO convert lua table with the right shape to an `UpdateSpec`, defaulting the broadcaster_id
            // TODO convert lua array (table) with the right shape to multiple `UpdateSpec`s
            
            // For reference
            //struct UpdateSpec<'update_body> {
                //broadcaster_id: UserId,
                //reward_id: RewardId,
                //body: UpdateCustomRewardBody<'update_body>,
            //}
            //
            //enum ApiCallSpec<'update_body> {
                //GetRewards(GetRewardsSpec),
                //Update(UpdateSpec<'update_body>)
            //}


            // TODO? additional options like parse JSON from file?
            calls
        },
    };

    perform_calls(&client, ApiCallsSpec { calls }, &user_token).await
}

struct GetRewardsSpec {
    broadcaster_id: UserId,
}

struct UpdateSpec<'update_body> {
    broadcaster_id: UserId,
    reward_id: RewardId,
    body: UpdateCustomRewardBody<'update_body>,
}

enum ApiCallSpec<'update_body> {
    GetRewards(GetRewardsSpec),
    Update(UpdateSpec<'update_body>)
}
use ApiCallSpec::*;

struct ApiCallsSpec<'update_body> {
    calls: Vec<ApiCallSpec<'update_body>>,
}

async fn perform_calls<'update_body, Client: HttpClient>(
    client: &HelixClient<'_, Client>, 
    ApiCallsSpec {calls}: ApiCallsSpec<'update_body>, 
    token: &UserToken
) -> Res<()> {
    for call in calls {
        match call {
            GetRewards(GetRewardsSpec { broadcaster_id }) => {
                let request = GetCustomRewardRequest::broadcaster_id(broadcaster_id);
                let response: Vec<CustomReward> = client.req_get(request, token).await?.data;
                dbg!(response);
            }
            Update(update_spec) => {
                let request = UpdateCustomRewardRequest::new(
                    update_spec.broadcaster_id,
                    update_spec.reward_id,
                );

                let body = update_spec.body;

                client.req_patch(request, body, token).await?;
            }
        }
    }
    
    Ok(())
}

fn authorize(AuthSpec {
    addr,
    addr_string,
    app_id,
    app_secret,
}: AuthSpec) -> Res<AccessToken> {

    use rand::{Rng, thread_rng};
    use rouille::{Server, Response};
    use std::sync::{Arc, Mutex};

    tracing::info!("got addr {addr:?}");

    let auth_state_key = thread_rng().gen::<u128>();

    #[derive(Debug, Default)]
    struct AuthState {
        user_token: String,
        // TODO? replace these bools with an enum.
        // Or are most of the 8 states valid?
        server_running: bool,
        can_close: bool,
        is_closed: bool,
    }

    let auth_state: Arc<Mutex<AuthState>> = Arc::new(
        Mutex::new(
            AuthState::default()
        )
    );

    // Start webserver in background thread
    {
        let auth_state = Arc::clone(&auth_state);
        let auth = Arc::clone(&auth_state);
        tokio::spawn(async move {
            tracing::info!("starting server at {addr:?}");

            let server = Server::new(addr, move |request| {
                tracing::info!("{request:?}");

                let expected = auth_state_key.to_string();
                let actual = request.get_param("state");

                if Some(expected) != actual {
                    let expected = auth_state_key.to_string();
                    tracing::info!("{expected} != {actual:?}");
                    return Response::text("Invalid state!".to_string())
                        .with_status_code(401);
                }

                if let Some(user_token) = request.get_param("code") {
                    tracing::info!("user_token: {user_token:?}");
                    auth.lock().expect("should not be poisoned").user_token = user_token;
                    let document: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <style type="text/css">body{
    margin:40px auto;
    max-width:650px;
    line-height:1.6;
    font-size:18px;
    color:#888;
    background-color:#111;
    padding:0 10px
    }
    h1{line-height:1.2}
    </style>
    <title>TART OAuth</title>
</head>
<body>
    <h1>Thanks for Authenticating with TART OAuth!</h1>
You may now close this page.
</body>
</html>"#;
                    Response::html(document)
                } else {
                    Response::text("must provide code").with_status_code(400)
                }
            });
            let auth = Arc::clone(&auth_state);
            auth.lock().expect("should not be poisoned").server_running = true;
            let server = server.expect("server startup error:");

            while !auth.lock().expect("should not be poisoned").can_close {
                server.poll();
                std::thread::sleep(SLEEP_DURATION);
            }

            auth.lock().expect("should not be poisoned").is_closed = true;
        });
    }

    let auth = Arc::clone(&auth_state);

    while !auth.lock().expect("should not be poisoned").server_running {
        std::thread::sleep(SLEEP_DURATION);
    }
    tracing::info!("Done waiting for server to start.");

    const TWITCH_AUTH_BASE_URL: &str = "https://id.twitch.tv/oauth2/";

    let auth_state_key_string = auth_state_key.to_string();

    let mut auth_url = Url::parse(
        TWITCH_AUTH_BASE_URL
    )?;
    auth_url = auth_url.join("authorize")?;
    auth_url.query_pairs_mut()
        .append_pair("client_id", &app_id)
        .append_pair("redirect_uri", &addr_string)
        .append_pair("response_type", "code")
        .append_pair("scope", "channel:manage:redemptions")
        .append_pair("force_verify", "true")
        .append_pair("state", &auth_state_key_string)
        ;

    tracing::info!("{}", auth_url.as_str());

    webbrowser::open(auth_url.as_str())?;

    tracing::info!("Waiting for auth confirmation.");

    while auth.lock().expect("should not be poisoned").user_token.is_empty() {
        std::thread::sleep(SLEEP_DURATION);
    }
    tracing::info!("Done waiting for auth confirmation.");

    let user_token = auth.lock().expect("should not be poisoned").user_token.clone();

    let mut token_url = Url::parse(
        TWITCH_AUTH_BASE_URL
    )?;
    token_url = token_url.join("token")?;
    token_url.query_pairs_mut()
        .append_pair("client_id", &app_id)
        .append_pair("client_secret", &app_secret)
        .append_pair("redirect_uri", &addr_string)
        .append_pair("code", &user_token)
        .append_pair("grant_type", "authorization_code")
        ;

    #[derive(serde::Deserialize)]
    struct Resp {
        access_token: String,
        refresh_token: String,
    }

    let Resp {
        access_token,
        refresh_token,
    }: Resp = ureq::post(token_url.as_str())
        .call()?
        .into_json::<Resp>()?;

    auth.lock().expect("should not be poisoned").can_close = true;

    tracing::info!("Waiting for server to close.");
    while !auth.lock().expect("should not be poisoned").is_closed {
        std::thread::sleep(SLEEP_DURATION);
    }
    tracing::info!("Done waiting for server to close.");

    if access_token.is_empty() {
        return Err("access_token was empty!".into());
    }

    tracing::info!("access_token: {access_token}");
    // TODO? use refresh token after a while?
    tracing::info!("refresh_token: {refresh_token}");

    Ok(AccessToken::new(access_token))
}