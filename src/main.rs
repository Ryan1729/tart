#![deny(unused_must_use)]


use std::{
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    time::{Duration}
};
use twitch_api::{
    HelixClient,
    HttpClient,
    helix::points::update_custom_reward::{
        UpdateCustomRewardBody,
        UpdateCustomRewardRequest
    },
};
use twitch_oauth2::tokens::UserToken;
use twitch_types::{
    points::RewardId,
    UserId
};
use url::Url;

const DEBUG_MODE: bool = cfg!(debug_assertions);

mod flags;

type Res<A> = Result<A, Box<dyn std::error::Error>>;

const SLEEP_DURATION: std::time::Duration = std::time::Duration::from_millis(16);

pub type Token = String;

pub enum SpecKind {
    Token(Token),
    Auth(AuthSpec)
}

pub struct Spec {
    pub channel_names: Vec<String>,
    pub login_name: String,
    pub kind: SpecKind,
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
        channel_names,
        login_name,
        kind,
    } = args.to_spec()?;

    tracing_subscriber::fmt::init();

    let oauth_token = match kind {
        SpecKind::Auth(auth_spec) => authorize(auth_spec)?,
        SpecKind::Token(token) => token,
    };

    tracing::info!("Debug mode: {DEBUG_MODE}");

    let agent: ureq::Agent = todo!();

    let client: HelixClient<ureq::Agent> = HelixClient::with_client(agent);

    let token = todo!(); //oauth_token

    // TODO allow specifying calls somehow. Parse JSON from file?
    perform_calls(&client, ApiCallsSpec { calls: vec![] }, &token).await
}

fn authorize(AuthSpec {
    addr,
    addr_string,
    app_id,
    app_secret,
}: AuthSpec) -> Res<String> {

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
    <title>TTTT OAuth</title>
</head>
<body>
    <h1>Thanks for Authenticating with TTTT OAuth!</h1>
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
        .append_pair("scope", "chat:read chat:edit")
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

    Ok(access_token)
}

struct UpdateSpec<'update_body> {
    broadcaster_id: UserId,
    reward_id: RewardId,
    body: UpdateCustomRewardBody<'update_body>,
}

enum ApiCallSpec<'update_body> {
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