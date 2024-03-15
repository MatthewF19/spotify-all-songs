use spotify_rs::{AuthCodeClient, AuthCodeFlow, model::PlayableItem::Track, RedirectUrl};

use url::Url;

use chrono::prelude::*;

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // set up authentication
    let redirect_url = RedirectUrl::new("http://localhost:8888/callback".to_owned())?;
    let auto_refresh = true;
    let scopes = vec!["playlist-modify-public", "user-library-read"];
    let auth_code_flow = AuthCodeFlow::new("17db9249d71d4fb9bea3135f40e154f1", "3efa5651ae1444ad8dff77a6732a304f", scopes);
    let (client, url) = AuthCodeClient::new(auth_code_flow, redirect_url.clone(), auto_refresh);
    // instruct user
    println!("go to this site and enter the url you're redirected to after signing in\n{}\n", url.as_str());

    // get auth url from user
    let mut auth_url_str = String::new();
    let _ = std::io::stdin().read_line(&mut auth_url_str);
    // extract auth code from url
    let auth_code = Url::parse(&auth_url_str)?.query_pairs().nth(0).unwrap().1.to_string();
    // get csrf token from url
    let csrf = url.query_pairs().nth(2).ok_or("invalid state")?.1.to_string();

    // authenticate
    let mut spotify = client.authenticate(auth_code.as_str(), csrf).await?;

    // get users' public playlists
    let person = spotify.current_user_playlists().get().await?;

    println!("\nWorking....");

    let mut all_songs_uris = Vec::new();
    // loop through all of my playlists
    for pl in person.items.iter() {
        // loop through all of the tracks in the playlist if they exist
        let tracks_id = &pl.id;
        if let Ok(x) = spotify.playlist(tracks_id).get().await {
            // loop through tracks
            for s in x.tracks.items.iter() {
                // only add if it's a track (not an episode)
                match &s.track {
                    Track(t) => all_songs_uris.push(t.uri.clone()),
                    _ => (),
                };
            }
        }
    }

    let mut x = 0;
    // total number of songs to add
    let max = spotify.saved_tracks().get().await?.total;
    while (x*50) < max {
        // get everything in liked songs
        let items = spotify.saved_tracks().limit(50).offset(x*50+1).get().await?.items;
        // add each liked song's uri to vector
        for item in items.iter() {
            all_songs_uris.push(item.track.uri.clone());
        }
        x += 1;
    }

    // all song ids
    let temp = all_songs_uris.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
    // nice title with current day / time
    let current_datetime = Local::now();
    let title = format!("All Songs <- {}", current_datetime.format("%Y-%m-%d %H:%M"));
    // get current user's id
    let current_user_id = spotify.get_current_user_profile().await?.id;
    // id of new playlist
    let all_songs_id = spotify.create_playlist(current_user_id, title).tracks(&temp[0..1]).send().await?.id;
    // add songs in increments of 50
    let mut idx = 0;
    while ((idx+1)*50) < temp.len() {
        spotify.add_items_to_playlist(all_songs_id.clone(), &temp[idx*50+1..((idx+1)*50+1)]).send().await?;
        idx += 1;
    }

    // add the remaining songs
    spotify.add_items_to_playlist(all_songs_id.clone(), &temp[idx*50+1..]).send().await?;

    println!("\nSuccess!!\n");

    Ok(())
}
