use std::env;
use serde::{Deserialize, Serialize};
use rspotify::{clients::{OAuthClient, BaseClient}, scopes, AuthCodeSpotify, Token, Credentials, model::{CurrentlyPlayingContext, PlayableItem, Image}};

#[derive(Deserialize, Serialize)]
pub struct SpotifyCurrentPlaying {
    name: String,
    artists: Vec<String>,
    album: String,
    album_images: Vec<Image>,
    is_playing: bool,
    progress: Option<u64>,
    song_duration: u64,
    timestamp: i64,
}

const ACCESS_TOKEN_KEY: &str = "SPOTIFY_ACCESS_TOKEN";
const REFRESH_TOKEN_KEY: &str = "SPOTIFY_REFRESH_TOKEN";
const CACHE_KEY: &str = "SPOTIFY_CACHE";
const CACHE_TIMESTAMP_KEY: &str = "SPOTIFY_CACHE_TIMEOUT";

pub async fn get_current_playing() -> Option<SpotifyCurrentPlaying> {
    let cache_attempt = get_current_playing_from_cache();

    if cache_attempt.is_some() {
        return cache_attempt;
    } else {
        return get_current_playing_from_spotify().await;
    }
}

fn build_token(access_token: &String, refresh_token: &String) -> Token {
    Token {
        access_token: access_token.clone(),
        refresh_token: Option::from(refresh_token.clone()),
        expires_at: None,
        scopes: scopes!("user-read-currently-playing"),
        expires_in: chrono::Duration::minutes(1),
    }
}

async fn get_current_playing_from_spotify() -> Option<SpotifyCurrentPlaying> {
    let access_token_opt = env::vars().find(|(key, _)| key == ACCESS_TOKEN_KEY);
    let refresh_token_opt = env::vars().find(|(key, _)| key == REFRESH_TOKEN_KEY);

    println!("Fetching from Spotify...");

    if access_token_opt.is_some() && refresh_token_opt.is_some() {
        let access_token = access_token_opt.unwrap().1;
        let refresh_token = refresh_token_opt.unwrap().1;

        let token = build_token(&access_token, &refresh_token);

        let mut spotify = AuthCodeSpotify::from_token(token);

        let current = spotify.current_playing(None, None::<&[_]>).await;

        if current.is_ok() {
            let current_unwrapped = &current.unwrap();
            if current_unwrapped.is_some() {
                return build_response(&current_unwrapped.clone().unwrap());
            }
        } else {
            let error = current.unwrap_err();

            if error.to_string().contains("401") {
                spotify.creds = Credentials::from_env().unwrap();
                let new_token_res = spotify.refetch_token().await;

                if new_token_res.is_ok() {
                    let new_token = new_token_res.unwrap().unwrap();

                    env::set_var(ACCESS_TOKEN_KEY, new_token.access_token.clone());

                    spotify = AuthCodeSpotify::from_token(new_token);

                    let current = spotify.current_playing(None, None::<&[_]>).await;

                    if current.is_ok() {
                        let current_unwrapped = &current.unwrap();
                        if current_unwrapped.is_some() {
                            return build_response(&current_unwrapped.clone().unwrap());
                        }
                    }
                } else {
                    panic!("Failed to refetch token: {:?}", new_token_res.err())
                }
            }
        }
    } else {
        panic!("Access Token/Refresh Token env var not found!");
    }

    None
}

fn get_current_playing_from_cache() -> Option<SpotifyCurrentPlaying> {
    let cache_timestamp_opt = env::vars().find(|(key, _)| key == CACHE_TIMESTAMP_KEY);
    let cache_opt = env::vars().find(|(key, _)| key == CACHE_KEY);

    if cache_timestamp_opt.is_some() && cache_opt.is_some() {
        let timestamp_opt = cache_timestamp_opt.unwrap().1.parse::<i64>();

        if timestamp_opt.is_ok() {
            let timestamp_cache = timestamp_opt.unwrap();
            let timestamp_now = chrono::offset::Utc::now().timestamp();

            if timestamp_now - timestamp_cache <= 10 {
                let cache_opt = env::vars().find(|(key, _)| key == CACHE_KEY);

                if cache_opt.is_some() {
                    let cache = cache_opt.unwrap().1;

                    let cached_result= serde_json::from_str::<Option<SpotifyCurrentPlaying>>(&cache);

                    if cached_result.is_ok() {
                        println!("Fetched from Cache");
                        return Option::from(cached_result.unwrap());
                    }
                }
            }
        } else {
            eprintln!("{:?}", timestamp_opt.err());
        }
    }

    None
}

fn build_response(context: &CurrentlyPlayingContext) -> Option<SpotifyCurrentPlaying> {
    let track = context.item.clone().unwrap();

    match track {
        PlayableItem::Track(track) => {
            let artists: Vec<String> = track.artists.iter().map(|a| a.name.clone()).collect();

            let progress: Option<u64> = match context.progress {
                Some(p) => Option::from(p.as_secs()),
                None => None
            };

            let timestamp = chrono::offset::Utc::now().timestamp();

            let response = Option::from(SpotifyCurrentPlaying {
                name: track.name,
                album: track.album.name,
                artists,
                album_images: track.album.images,
                is_playing: context.is_playing,
                progress,
                song_duration: track.duration.as_secs(),
                timestamp
            });

            let serialized = serde_json::to_string(&response);

            if serialized.is_ok() {
                env::set_var(CACHE_KEY, serialized.unwrap());
                env::set_var(CACHE_TIMESTAMP_KEY, timestamp.to_string());
            } else {
                eprintln!("{:?}", serialized.err());
            }

            response
        },
        PlayableItem::Episode(_) => None
    }
}
