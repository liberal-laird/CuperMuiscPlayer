use anyhow::Result;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::collections::VecDeque;
use std::fs;
use std::io::BufReader;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Song {
    pub path: PathBuf,
    pub name: String,
    pub duration: Option<Duration>,
}

#[derive(Debug, PartialEq)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

pub struct App {
    pub songs: Vec<Song>,
    pub current_index: usize,
    pub playback_state: PlaybackState,
    pub current_time: Duration,
    pub volume: f32,
    pub is_shuffle: bool,
    pub shuffle_history: VecDeque<usize>,
    
    // Rodio components
    pub _stream: OutputStream,
    pub _stream_handle: OutputStreamHandle,
    pub sink: Option<Sink>,
    
    // Progress tracking
    pub play_start_time: Option<std::time::Instant>,
    pub current_play_time: Duration,
}

impl App {
    pub fn new() -> Result<Self> {
        let (_stream, _stream_handle) = OutputStream::try_default()?;
        
        let mut app = App {
            songs: Vec::new(),
            current_index: 0,
            playback_state: PlaybackState::Stopped,
            current_time: Duration::ZERO,
            volume: 0.5,
            is_shuffle: false,
            shuffle_history: VecDeque::new(),
            _stream,
            _stream_handle,
            sink: None,
            play_start_time: None,
            current_play_time: Duration::ZERO,
        };
        
        app.load_songs()?;
        Ok(app)
    }
    
    fn get_audio_duration(path: &PathBuf) -> Option<Duration> {
        let src = match std::fs::File::open(path) {
            Ok(file) => MediaSourceStream::new(Box::new(file), Default::default()),
            Err(_) => return None,
        };

        let mut hint = Hint::new();
        if let Some(extension) = path.extension() {
            if let Some(extension_str) = extension.to_str() {
                hint.with_extension(extension_str);
            }
        }

        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        let probed = match symphonia::default::get_probe().format(&hint, src, &fmt_opts, &meta_opts) {
            Ok(probed) => probed,
            Err(_) => return None,
        };

        let format = probed.format;
        let track = match format.tracks().iter().next() {
            Some(track) => track,
            None => return None,
        };

        let time_base = track.codec_params.time_base;
        let duration = track.codec_params.n_frames;

        if let (Some(tb), Some(n_frames)) = (time_base, duration) {
            let time = tb.calc_time(n_frames);
            // 修复时间计算：使用正确的秒数计算
            let total_seconds = time.seconds as f64 + (time.frac as f64 / tb.denom as f64);
            Some(Duration::from_secs_f64(total_seconds))
        } else {
            None
        }
    }

    fn load_songs(&mut self) -> Result<()> {
        let assets_dir = PathBuf::from("assets");
        if !assets_dir.exists() {
            return Ok(());
        }
        
        for entry in fs::read_dir(assets_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if let Some(extension) = path.extension() {
                let ext = extension.to_string_lossy().to_lowercase();
                if matches!(ext.as_str(), "mp3" | "wav" | "flac" | "ogg" | "m4a" | "mp4a") {
                    let name = path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    
                    let duration = Self::get_audio_duration(&path);
                    
                    self.songs.push(Song {
                        path,
                        name,
                        duration,
                    });
                }
            }
        }
        
        Ok(())
    }
    
    pub fn play(&mut self) -> Result<()> {
        if self.songs.is_empty() {
            return Ok(());
        }
        
        self.stop()?;
        
        let song = &self.songs[self.current_index];
        
        let file = fs::File::open(&song.path)?;
        let reader = BufReader::new(file);
        
        let sink = Sink::try_new(&self._stream_handle)?;
        
        // Try to decode with rodio decoder
        match Decoder::new(reader) {
            Ok(decoder) => {
                sink.append(decoder);
                sink.set_volume(self.volume);
                sink.play();
                
                self.sink = Some(sink);
                self.playback_state = PlaybackState::Playing;
                self.play_start_time = Some(std::time::Instant::now());
                self.current_play_time = Duration::ZERO;
            }
            Err(_) => {
                // 解码失败，尝试下一个文件
                if self.songs.len() > 1 {
                    self.next_without_play()?;
                    self.play()?;
                }
            }
        }
        
        Ok(())
    }
    
    pub fn pause(&mut self) {
        if let Some(ref sink) = self.sink {
            sink.pause();
            self.playback_state = PlaybackState::Paused;
            // 保存当前播放时间
            if let Some(start_time) = self.play_start_time {
                self.current_play_time = start_time.elapsed();
            }
        }
    }
    
    pub fn resume(&mut self) {
        if let Some(ref sink) = self.sink {
            sink.play();
            self.playback_state = PlaybackState::Playing;
            // 重新设置开始时间，考虑已经播放的时间
            let elapsed = self.current_play_time;
            self.play_start_time = Some(std::time::Instant::now() - elapsed);
        }
    }
    
    pub fn stop(&mut self) -> Result<()> {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.playback_state = PlaybackState::Stopped;
        self.current_time = Duration::ZERO;
        self.play_start_time = None;
        self.current_play_time = Duration::ZERO;
        Ok(())
    }
    
    pub fn next(&mut self) -> Result<()> {
        if self.songs.is_empty() {
            return Ok(());
        }
        
        if self.is_shuffle {
            self.next_shuffle();
        } else {
            self.current_index = (self.current_index + 1) % self.songs.len();
        }
        
        self.play()?;
        Ok(())
    }
    
    pub fn next_without_play(&mut self) -> Result<()> {
        if self.songs.is_empty() {
            return Ok(());
        }
        
        if self.is_shuffle {
            self.next_shuffle();
        } else {
            self.current_index = (self.current_index + 1) % self.songs.len();
        }
        
        Ok(())
    }
    
    pub fn previous(&mut self) -> Result<()> {
        if self.songs.is_empty() {
            return Ok(());
        }
        
        if self.current_index == 0 {
            self.current_index = self.songs.len() - 1;
        } else {
            self.current_index -= 1;
        }
        
        self.play()?;
        Ok(())
    }
    
    pub fn toggle_shuffle(&mut self) {
        self.is_shuffle = !self.is_shuffle;
        if self.is_shuffle {
            self.shuffle_history.clear();
        }
    }
    
    fn next_shuffle(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        if self.shuffle_history.len() >= self.songs.len() {
            self.shuffle_history.clear();
        }
        
        let mut next_index;
        loop {
            next_index = rng.gen_range(0..self.songs.len());
            if !self.shuffle_history.contains(&next_index) {
                break;
            }
        }
        
        self.shuffle_history.push_back(self.current_index);
        self.current_index = next_index;
    }
    
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.max(0.0).min(1.0);
        if let Some(ref sink) = self.sink {
            sink.set_volume(self.volume);
        }
    }
    
    pub fn get_current_song(&self) -> Option<&Song> {
        self.songs.get(self.current_index)
    }
    
    pub fn check_and_auto_next(&mut self) -> Result<()> {
        if let Some(ref sink) = self.sink {
            // 检查 sink 是否为空且不在暂停状态（播放结束）
            if sink.len() == 0 && !sink.is_paused() && self.playback_state == PlaybackState::Playing {
                // 播放结束，自动播放下一曲
                if self.songs.len() > 1 {
                    self.next()?;
                } else {
                    // 只有一首歌，重新播放
                    self.play()?;
                }
            }
        }
        Ok(())
    }
    
    pub fn update_play_time(&mut self) {
        match self.playback_state {
            PlaybackState::Playing => {
                if let Some(start_time) = self.play_start_time {
                    let elapsed = start_time.elapsed();
                    // 确保播放时间不超过总时长
                    let total_duration = self.get_total_duration();
                    if elapsed > total_duration {
                        self.current_play_time = total_duration;
                    } else {
                        self.current_play_time = elapsed;
                    }
                }
            }
            PlaybackState::Paused => {
                // 暂停时不更新播放时间，保持当前值
            }
            PlaybackState::Stopped => {
                // 停止时重置播放时间
                self.current_play_time = Duration::ZERO;
            }
        }
    }
    
    pub fn get_current_time(&self) -> Duration {
        match self.playback_state {
            PlaybackState::Playing => {
                if let Some(start_time) = self.play_start_time {
                    let elapsed = start_time.elapsed();
                    // 确保播放时间不超过总时长
                    let total_duration = self.get_total_duration();
                    if elapsed > total_duration {
                        return total_duration;
                    }
                    return elapsed;
                }
                self.current_play_time
            }
            PlaybackState::Paused => {
                // 暂停时返回保存的播放时间
                self.current_play_time
            }
            PlaybackState::Stopped => {
                Duration::ZERO
            }
        }
    }
    
    pub fn get_total_duration(&self) -> Duration {
        if let Some(song) = self.get_current_song() {
            song.duration.unwrap_or(Duration::from_secs(180)) // 默认3分钟
        } else {
            Duration::from_secs(180)
        }
    }
    
    pub fn get_progress(&self) -> f32 {
        let current_time = self.get_current_time();
        let total_duration = self.get_total_duration();
        
        if total_duration.as_secs() > 0 {
            let progress = (current_time.as_secs_f64() / total_duration.as_secs_f64()).min(1.0).max(0.0) as f32;
            return progress;
        }
        0.0
    }
} 