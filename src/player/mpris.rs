use mpris_server::{
    Metadata, PlaybackStatus,Server
};
use std::sync::{Arc, Mutex};
use flume::{Sender, Receiver};

use crate::player::{messages::{MprisCommand, MprisUpdate, PlaybackState}, player::MyPlayer};

pub fn start_mpris(
    update_rx: Receiver<MprisUpdate>,
    cmd_tx: Sender<MprisCommand>,
) {
    std::thread::spawn(move || {
        async_std::task::block_on(async move {
            let player = MyPlayer {
                state: Arc::new(Mutex::new(PlaybackState::Stopped)),
                cmd_tx,
            };

            let server = Server::new("com.linn.player", player).await.unwrap();

            loop {
                if let Ok(update) = update_rx.recv() {
                    match update {
                        MprisUpdate::PlaybackState(state) => {
                            server
                                .properties_changed([mpris_server::Property::PlaybackStatus(
                                    match state {
                                        PlaybackState::Playing => PlaybackStatus::Playing,
                                        PlaybackState::Paused => PlaybackStatus::Paused,
                                        PlaybackState::Stopped => PlaybackStatus::Stopped,
                                        PlaybackState::Buffering => PlaybackStatus::Stopped,
                                    }
                                )])
                                .await
                                .ok();
                        }
                        MprisUpdate::Metadata(song) => {
                            let artists=song.artists.iter().map(|a| a.name.clone()).collect::<Vec<String>>();
                            let metadata = Metadata::builder()
                                .title(song.name)
                                .artist(artists)
                                .album(song.album.name)
                                .art_url(song.cover_url)
                                .build();

                            server
                                .properties_changed([
                                    mpris_server::Property::Metadata(metadata)
                                ])
                                .await
                                .ok();
                        }
                    }
                }
            }
        });
    });
}
