use std::sync::{Arc, Mutex};

use flume::Sender;
use mpris_server::{
    LoopStatus, Metadata, PlaybackStatus, PlayerInterface, RootInterface, Time, Volume, zbus::fdo,
};

use crate::player::messages::{MprisCommand, PlayMode, PlaybackState};

pub struct MyPlayer {
    pub(crate) state: Arc<Mutex<PlaybackState>>,
    pub(crate) current_metadata: Arc<Mutex<Metadata>>, // 增加元数据缓存
    pub(crate) current_position_ms: Arc<Mutex<u64>>,
    pub(crate) playback_settings: Arc<Mutex<(PlayMode, bool)>>,
    pub(crate) cmd_tx: Sender<MprisCommand>,
}

impl RootInterface for MyPlayer {
    async fn identity(&self) -> fdo::Result<String> {
        Ok("Linn Player".into())
    }

    async fn raise(&self) -> fdo::Result<()> {
        Ok(())
    }

    async fn quit(&self) -> fdo::Result<()> {
        Ok(())
    }

    async fn can_quit(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn set_fullscreen(&self, _fullscreen: bool) -> mpris_server::zbus::Result<()> {
        Ok(())
    }

    async fn can_set_fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn can_raise(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn has_track_list(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn desktop_entry(&self) -> fdo::Result<String> {
        Ok("linn-player".into()) // 👉 对应 .desktop 文件名（不带 .desktop）
    }

    async fn supported_uri_schemes(&self) -> fdo::Result<Vec<String>> {
        Ok(vec!["http".into(), "https".into()])
    }

    async fn supported_mime_types(&self) -> fdo::Result<Vec<String>> {
        Ok(vec!["audio/mpeg".into(), "audio/flac".into()])
    }
}

//

//
impl PlayerInterface for MyPlayer {
    async fn play(&self) -> fdo::Result<()> {
        self.cmd_tx.send(MprisCommand::Play).ok();
        Ok(())
    }

    async fn pause(&self) -> fdo::Result<()> {
        self.cmd_tx.send(MprisCommand::Pause).ok();
        Ok(())
    }
    async fn play_pause(&self) -> fdo::Result<()> {
        let current_state = *self.state.lock().unwrap();
        match current_state {
            PlaybackState::Playing => {
                self.cmd_tx.send(MprisCommand::Pause).ok();
            }
            _ => {
                self.cmd_tx.send(MprisCommand::Play).ok();
            }
        }
        Ok(())
    }
    async fn stop(&self) -> fdo::Result<()> {
        Ok(())
    }

    async fn next(&self) -> fdo::Result<()> {
        self.cmd_tx.send(MprisCommand::Next).ok();
        Ok(())
    }
    async fn previous(&self) -> fdo::Result<()> {
        self.cmd_tx.send(MprisCommand::Previous).ok();
        Ok(())
    }

    async fn seek(&self, offset: Time) -> fdo::Result<()> {
        self.cmd_tx
            .send(MprisCommand::SeekRelative(offset.as_millis()))
            .ok();
        Ok(())
    }
    async fn set_position(&self, track_id: mpris_server::TrackId, pos: Time) -> fdo::Result<()> {
        // MPRIS 要求忽略已过期歌曲的 SetPosition 请求，防止切歌竞态把新歌跳转。
        let is_current_track = self
            .current_metadata
            .lock()
            .unwrap()
            .trackid()
            .is_some_and(|current_track| current_track == track_id);
        if is_current_track && pos.as_millis() >= 0 {
            self.cmd_tx
                .send(MprisCommand::SetPosition(pos.as_millis() as u64))
                .ok();
        }
        Ok(())
    }

    async fn open_uri(&self, _uri: String) -> fdo::Result<()> {
        Ok(())
    }

    // ===== 状态 =====

    async fn playback_status(&self) -> fdo::Result<PlaybackStatus> {
        let state = *self.state.lock().unwrap();
        Ok(match state {
            PlaybackState::Playing => PlaybackStatus::Playing,
            PlaybackState::Paused => PlaybackStatus::Paused,
            PlaybackState::Stopped => PlaybackStatus::Stopped,
            PlaybackState::Buffering => PlaybackStatus::Stopped,
        })
    }

    async fn loop_status(&self) -> fdo::Result<LoopStatus> {
        let (play_mode, loop_enabled) = *self.playback_settings.lock().unwrap();
        Ok(match play_mode {
            PlayMode::SingleLoop => LoopStatus::Track,
            _ if loop_enabled => LoopStatus::Playlist,
            _ => LoopStatus::None,
        })
    }

    async fn set_loop_status(&self, status: LoopStatus) -> mpris_server::zbus::Result<()> {
        self.cmd_tx.send(MprisCommand::SetLoopStatus(status)).ok();
        Ok(())
    }

    async fn rate(&self) -> fdo::Result<f64> {
        Ok(1.0)
    }

    async fn set_rate(&self, _rate: f64) -> mpris_server::zbus::Result<()> {
        Ok(())
    }

    async fn shuffle(&self) -> fdo::Result<bool> {
        Ok(matches!(
            self.playback_settings.lock().unwrap().0,
            PlayMode::Shuffle
        ))
    }

    async fn set_shuffle(&self, shuffle: bool) -> mpris_server::zbus::Result<()> {
        self.cmd_tx.send(MprisCommand::SetShuffle(shuffle)).ok();
        Ok(())
    }

    async fn metadata(&self) -> fdo::Result<Metadata> {
        Ok(self.current_metadata.lock().unwrap().clone())
    }

    async fn volume(&self) -> fdo::Result<Volume> {
        Ok(1.0)
    }

    async fn set_volume(&self, _volume: Volume) -> mpris_server::zbus::Result<()> {
        Ok(())
    }

    async fn position(&self) -> fdo::Result<Time> {
        Ok(Time::from_millis(
            (*self.current_position_ms.lock().unwrap()).min(i64::MAX as u64) as i64,
        ))
    }

    async fn minimum_rate(&self) -> fdo::Result<f64> {
        Ok(1.0)
    }

    async fn maximum_rate(&self) -> fdo::Result<f64> {
        Ok(1.0)
    }

    async fn can_go_next(&self) -> fdo::Result<bool> {
        Ok(true)
    }
    async fn can_go_previous(&self) -> fdo::Result<bool> {
        Ok(true)
    }
    async fn can_play(&self) -> fdo::Result<bool> {
        Ok(true)
    }
    async fn can_pause(&self) -> fdo::Result<bool> {
        Ok(true)
    }
    async fn can_seek(&self) -> fdo::Result<bool> {
        Ok(true)
    }
    async fn can_control(&self) -> fdo::Result<bool> {
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flume::Receiver;
    use mpris_server::TrackId;

    fn track_id(id: u64) -> TrackId {
        TrackId::try_from(format!("/com/linn/player/tracks/{id}")).unwrap()
    }

    fn player(track_id: TrackId) -> (MyPlayer, Receiver<MprisCommand>) {
        let (cmd_tx, cmd_rx) = flume::unbounded();
        let player = MyPlayer {
            state: Arc::new(Mutex::new(PlaybackState::Stopped)),
            current_metadata: Arc::new(Mutex::new(Metadata::builder().trackid(track_id).build())),
            current_position_ms: Arc::new(Mutex::new(12_345)),
            playback_settings: Arc::new(Mutex::new((PlayMode::Sequential, true))),
            cmd_tx,
        };
        (player, cmd_rx)
    }

    #[test]
    fn set_position_sends_absolute_position_for_current_track() {
        let track_id = track_id(1);
        let (player, cmd_rx) = player(track_id.clone());

        async_std::task::block_on(player.set_position(track_id, Time::from_millis(42_000)))
            .unwrap();

        assert!(matches!(
            cmd_rx.try_recv(),
            Ok(MprisCommand::SetPosition(42_000))
        ));
    }

    #[test]
    fn set_position_ignores_stale_track_and_negative_position() {
        let (player, cmd_rx) = player(track_id(1));

        async_std::task::block_on(player.set_position(track_id(2), Time::from_millis(42_000)))
            .unwrap();
        async_std::task::block_on(player.set_position(track_id(1), Time::from_millis(-1))).unwrap();

        assert!(cmd_rx.try_recv().is_err());
    }

    #[test]
    fn seek_keeps_mpris_relative_offset_in_milliseconds() {
        let (player, cmd_rx) = player(track_id(1));

        async_std::task::block_on(player.seek(Time::from_millis(-5_000))).unwrap();

        assert!(matches!(
            cmd_rx.try_recv(),
            Ok(MprisCommand::SeekRelative(-5_000))
        ));
    }

    #[test]
    fn position_returns_cached_engine_position() {
        let (player, _cmd_rx) = player(track_id(1));

        let position = async_std::task::block_on(player.position()).unwrap();

        assert_eq!(position.as_millis(), 12_345);
    }

    #[test]
    fn playback_settings_are_exposed_as_mpris_loop_and_shuffle() {
        let (player, _cmd_rx) = player(track_id(1));
        *player.playback_settings.lock().unwrap() = (PlayMode::SingleLoop, false);
        assert_eq!(
            async_std::task::block_on(player.loop_status()).unwrap(),
            LoopStatus::Track
        );
        assert!(!async_std::task::block_on(player.shuffle()).unwrap());

        *player.playback_settings.lock().unwrap() = (PlayMode::Shuffle, true);
        assert_eq!(
            async_std::task::block_on(player.loop_status()).unwrap(),
            LoopStatus::Playlist
        );
        assert!(async_std::task::block_on(player.shuffle()).unwrap());
    }

    #[test]
    fn mpris_playback_setting_setters_forward_commands() {
        let (player, cmd_rx) = player(track_id(1));

        async_std::task::block_on(player.set_loop_status(LoopStatus::Playlist)).unwrap();
        async_std::task::block_on(player.set_shuffle(true)).unwrap();

        assert!(matches!(
            cmd_rx.try_recv(),
            Ok(MprisCommand::SetLoopStatus(LoopStatus::Playlist))
        ));
        assert!(matches!(
            cmd_rx.try_recv(),
            Ok(MprisCommand::SetShuffle(true))
        ));
    }
}
