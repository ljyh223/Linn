
use std::sync::{Arc, Mutex};

use flume::Sender;
use mpris_server::{
    zbus::fdo,
    Metadata, PlaybackStatus, PlayerInterface, RootInterface,
    LoopStatus, Time, Volume,
};

use crate::player::messages::{MprisCommand, PlaybackState, PlayerCommand};

pub struct MyPlayer {
    pub(crate) state: Arc<Mutex<PlaybackState>>,
    pub(crate) current_metadata: Arc<Mutex<Metadata>>, // 增加元数据缓存
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
    async fn stop(&self) -> fdo::Result<()> { Ok(()) }

    async fn next(&self) -> fdo::Result<()> {
        self.cmd_tx.send(MprisCommand::Next).ok();
        Ok(())
    }
    async fn previous(&self) -> fdo::Result<()> {
        self.cmd_tx.send(MprisCommand::Previous).ok();
        Ok(())
    }

    async fn seek(&self, offset: Time) -> fdo::Result<()> {
        self.cmd_tx.send(MprisCommand::Seek(offset.as_micros() / 1000)).ok();
        Ok(())
    }
    async fn set_position(&self, _track_id: mpris_server::TrackId, _pos: Time) -> fdo::Result<()> { Ok(()) }

    async fn open_uri(&self, _uri: String) -> fdo::Result<()> { Ok(()) }

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
        Ok(LoopStatus::None)
    }

    async fn set_loop_status(&self, _status: LoopStatus) -> mpris_server::zbus::Result<()> {
        Ok(())
    }

    async fn rate(&self) -> fdo::Result<f64> {
        Ok(1.0)
    }

    async fn set_rate(&self, _rate: f64) -> mpris_server::zbus::Result<()> {
        Ok(())
    }

    async fn shuffle(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn set_shuffle(&self, _shuffle: bool) -> mpris_server::zbus::Result<()> {
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

    async fn position(&self) -> fdo::Result<Time> { Ok(Time::from_micros(0)) }

    async fn minimum_rate(&self) -> fdo::Result<f64> {
        Ok(1.0)
    }

    async fn maximum_rate(&self) -> fdo::Result<f64> {
        Ok(1.0)
    }

    async fn can_go_next(&self) -> fdo::Result<bool> { Ok(true) }
    async fn can_go_previous(&self) -> fdo::Result<bool> { Ok(true) }
    async fn can_play(&self) -> fdo::Result<bool> { Ok(true) }
    async fn can_pause(&self) -> fdo::Result<bool> { Ok(true) }
    async fn can_seek(&self) -> fdo::Result<bool> { Ok(true) }
    async fn can_control(&self) -> fdo::Result<bool> { Ok(true) }
}