use flume::{Receiver, Sender};
use mpris_server::{LoopStatus, Metadata, PlaybackStatus, Server, Time, TrackId}; // 引入 Time 和 TrackId
use std::sync::{Arc, Mutex};

use crate::player::{
    messages::{MprisCommand, MprisUpdate, PlayMode, PlaybackState},
    player::MyPlayer,
};

pub fn start_mpris(update_rx: Receiver<MprisUpdate>, cmd_tx: Sender<MprisCommand>) {
    std::thread::spawn(move || {
        async_std::task::block_on(async move {
            // 创建共享状态
            let shared_state = Arc::new(Mutex::new(PlaybackState::Stopped));
            let shared_metadata = Arc::new(Mutex::new(Metadata::builder().build()));
            let shared_position = Arc::new(Mutex::new(0_u64));
            let shared_playback_settings = Arc::new(Mutex::new((PlayMode::Sequential, true)));

            let player = MyPlayer {
                state: shared_state.clone(),
                current_metadata: shared_metadata.clone(), // 新增
                current_position_ms: shared_position.clone(),
                playback_settings: shared_playback_settings.clone(),
                cmd_tx,
            };

            let server = Server::new("com.linn.player", player).await.unwrap();

            loop {
                match update_rx.try_recv() {
                    Ok(update) => {
                        match update {
                            MprisUpdate::PlaybackState(state) => {
                                // 1. 更新本地状态缓存
                                *shared_state.lock().unwrap() = state.clone();

                                // 2. 通知总线状态已改变
                                server
                                    .properties_changed([mpris_server::Property::PlaybackStatus(
                                        match state {
                                            PlaybackState::Playing => PlaybackStatus::Playing,
                                            PlaybackState::Paused => PlaybackStatus::Paused,
                                            PlaybackState::Stopped => PlaybackStatus::Stopped,
                                            PlaybackState::Buffering => PlaybackStatus::Stopped,
                                        },
                                    )])
                                    .await
                                    .ok();
                            }
                            MprisUpdate::Metadata(song) => {
                                let artists = song
                                    .artists
                                    .iter()
                                    .map(|a| a.name.clone())
                                    .collect::<Vec<String>>();

                                let length_micros = (song.duration as i64) * 1000;
                                let track_id = TrackId::try_from(format!(
                                    "/com/linn/player/tracks/{}",
                                    song.id
                                ))
                                .unwrap();

                                let metadata = Metadata::builder()
                                    .trackid(track_id)
                                    .title(song.name)
                                    .artist(artists)
                                    .album(song.album.name)
                                    .art_url(song.cover_url)
                                    .length(Time::from_micros(length_micros)) // 写入时长
                                    .build();

                                // 1. 更新本地缓存
                                *shared_metadata.lock().unwrap() = metadata.clone();
                                *shared_position.lock().unwrap() = 0;

                                // 2. 通知总线
                                server
                                    .properties_changed([mpris_server::Property::Metadata(
                                        metadata,
                                    )])
                                    .await
                                    .ok();
                            }
                            MprisUpdate::Position(position_ms) => {
                                *shared_position.lock().unwrap() = position_ms;
                            }
                            MprisUpdate::PlaybackSettings {
                                play_mode,
                                loop_enabled,
                            } => {
                                *shared_playback_settings.lock().unwrap() =
                                    (play_mode, loop_enabled);
                                let loop_status = match play_mode {
                                    PlayMode::SingleLoop => LoopStatus::Track,
                                    _ if loop_enabled => LoopStatus::Playlist,
                                    _ => LoopStatus::None,
                                };
                                server
                                    .properties_changed([
                                        mpris_server::Property::LoopStatus(loop_status),
                                        mpris_server::Property::Shuffle(matches!(
                                            play_mode,
                                            PlayMode::Shuffle
                                        )),
                                    ])
                                    .await
                                    .ok();
                            }
                        }
                    }
                    Err(flume::TryRecvError::Empty) => {
                        async_std::task::sleep(std::time::Duration::from_millis(10)).await;
                    }
                    Err(flume::TryRecvError::Disconnected) => break,
                }
            }
        });
    });
}
