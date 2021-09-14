use std::io::Cursor;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::Bytes;
use rodio::source::Buffered;
use rodio::{queue, Source};
use rodio::{Decoder, OutputStream, OutputStreamHandle};

#[derive(Debug, Clone)]
pub enum PlayerError {
    DecodeTrackError(String),
}

pub trait AudioPlayer {
    /// play a sound track
    fn play(&mut self, track: Bytes) -> Result<(), PlayerError>;
    /// stop play
    fn stop(&self);
    /// pause the audio player
    fn pause(&self);
    /// resume to play
    fn resume(&self);
    /// increase volume
    fn increase_volume(&self, delta: u8) -> u8;
    /// decrease volume
    fn decrease_volume(&self, delta: u8) -> u8;
    /// slide progress bar
    fn seek_ms(&self, progress_ms: u64);
    /// forward (millisecond)
    fn forward(&self, dur_millis: u64);
    /// rewind (millisecond)
    fn rewind(&self, dur_millis: u64);
    /// change speed (millisecond)
    fn speed(&self, speed: f32);
    /// current playback context
    fn playback_context(&self) -> Arc<PlaybackContext>;
}

pub struct PlaybackContext {
    pause: AtomicBool,
    stopped: AtomicBool,
    volume: Mutex<f32>,
    speed: Mutex<f32>,
    // milliseconds of the progress bar
    progress_ms: AtomicU64,
    progress_interval_ms: AtomicU64,
}

pub(crate) struct LAudioPlayer {
    queue_tx: Arc<queue::SourcesQueueInput<f32>>,
    end_signal: Mutex<Option<Receiver<()>>>,

    playback_context: Arc<PlaybackContext>,

    detached: bool,
    _output_stream_handle: OutputStreamHandle,
    _output_stream: OutputStream,
    current_track: Option<Buffered<Decoder<Cursor<Bytes>>>>,
}

impl LAudioPlayer {
    #[inline]
    pub fn try_new() -> Result<Self, Box<dyn std::error::Error>> {
        let (player, queue_rx) = Self::new_idle();
        player._output_stream_handle.play_raw(queue_rx)?;
        Ok(player)
    }

    #[inline]
    pub fn new_idle() -> (Self, queue::SourcesQueueOutput<f32>) {
        let (stream, handle) = rodio::OutputStream::try_default().unwrap();
        let (queue_tx, queue_rx) = queue::queue(true);
        let player = Self {
            queue_tx,
            end_signal: Mutex::new(None),

            playback_context: Arc::new(PlaybackContext {
                pause: AtomicBool::new(false),
                stopped: AtomicBool::new(false),
                volume: Mutex::new(1.0),
                speed: Mutex::new(1.0),
                progress_ms: AtomicU64::new(0),
                progress_interval_ms: AtomicU64::new(5),
            }),
            detached: false,

            _output_stream: stream,
            _output_stream_handle: handle,
            current_track: None,
        };
        (player, queue_rx)
    }

    #[inline]
    fn set_current_track(&mut self, track: Bytes) -> Result<(), PlayerError> {
        let buf = Cursor::new(track);
        let source = Decoder::new(buf).map_err(|e| PlayerError::DecodeTrackError(e.to_string()))?;

        self.current_track = Some(source.buffered());
        Ok(())
    }

    #[inline]
    fn start_play(&self) {
        let context = self.playback_context.clone();

        if let Some(ref source) = self.current_track {
            // clip source by progress cursor
            let source = source
                .clone()
                .skip_duration(Duration::from_millis(
                    context.progress_ms.load(Ordering::SeqCst),
                ))
                .speed(*context.speed.lock().unwrap());

            let source = source
                .pausable(false)
                .amplify(1.0)
                .stoppable()
                .periodic_access(
                    Duration::from_millis(context.progress_interval_ms.load(Ordering::Relaxed)),
                    move |src| {
                        if context.stopped.load(Ordering::SeqCst) {
                            return src.stop();
                        }

                        src.inner_mut().set_factor(*context.volume.lock().unwrap());

                        let paused = context.pause.load(Ordering::SeqCst);
                        src.inner_mut().inner_mut().set_paused(paused);

                        if !paused {
                            context.progress_ms.fetch_add(5, Ordering::Relaxed);
                        }
                    },
                )
                .convert_samples();
            *self.end_signal.lock().unwrap() = Some(self.queue_tx.append_with_signal(source));
        }
    }

    /// The value `1.0` is the "normal" volume (unfiltered input). Any value other than 1.0 will
    /// multiply each sample by this value.
    #[inline]
    pub fn volume(&self) -> f32 {
        *self.playback_context.volume.lock().unwrap()
    }

    #[inline]
    pub fn set_volume(&self, value: f32) {
        *self.playback_context.volume.lock().unwrap() = value;
    }

    #[inline]
    pub fn set_speed(&self, value: f32) {
        *self.playback_context.speed.lock().unwrap() = value;
    }

    /// Resumes playback of a paused sink.
    /// No effect if not paused.
    #[inline]
    pub fn resume(&self) {
        self.playback_context.pause.store(false, Ordering::SeqCst);
    }

    /// Pauses playback of this sink.
    pub fn pause(&self) {
        self.playback_context.pause.store(true, Ordering::SeqCst);
    }

    /// Gets if a sink is paused
    pub fn is_paused(&self) -> bool {
        self.playback_context.pause.load(Ordering::SeqCst)
    }

    /// Stops the sink by emptying the queue.
    #[inline]
    pub fn drain_sink(&self) {
        self.playback_context.stopped.store(true, Ordering::SeqCst);
    }

    /// Replay the current track but with new playback_context
    pub fn replay(&self) {
        self.drain_sink();
        self.sleep_until_end();
        self.playback_context.stopped.store(false, Ordering::SeqCst);
        self.start_play();
    }
    /// Destroys the sink without stopping the sounds that are still playing.
    #[inline]
    pub fn detach(mut self) {
        self.detached = true;
    }

    /// Sleeps the current thread until the sound ends.
    #[inline]
    pub fn sleep_until_end(&self) {
        if let Some(end_signal) = self.end_signal.lock().unwrap().take() {
            let _ = end_signal.recv();
        }
    }
}

impl Drop for LAudioPlayer {
    #[inline]
    fn drop(&mut self) {
        self.queue_tx.set_keep_alive_if_empty(false);

        if !self.detached {
            self.playback_context.stopped.store(true, Ordering::Relaxed);
        }
    }
}

impl AudioPlayer for LAudioPlayer {
    fn play(&mut self, track: Bytes) -> Result<(), PlayerError> {
        self.playback_context.progress_ms.store(0, Ordering::SeqCst);
        self.set_current_track(track)?;
        self.start_play();
        Ok(())
    }

    fn pause(&self) {
        self.pause();
    }

    fn resume(&self) {
        self.resume();
    }

    fn stop(&self) {
        self.drain_sink();
    }

    fn increase_volume(&self, delta: u8) -> u8 {
        let vol = self.volume();
        if vol >= 2.0 {
            return (vol * 100.0) as u8;
        }

        let new_vol = delta as f32 / 100.0 + vol;
        self.set_volume(new_vol);
        (new_vol * 100.0) as u8
    }

    fn decrease_volume(&self, delta: u8) -> u8 {
        let vol = self.volume();
        if vol <= 0.0 {
            return 0;
        }

        let new_vol = vol - (delta as f32 / 100.0);
        self.set_volume(new_vol);
        (new_vol * 100.0) as u8
    }

    fn seek_ms(&self, progress_ms: u64) {
        self.playback_context
            .progress_ms
            .store(progress_ms, Ordering::SeqCst);
        self.replay();
    }

    fn forward(&self, dur_millis: u64) {
        self.playback_context
            .progress_ms
            .fetch_add(dur_millis, Ordering::SeqCst);
        self.replay();
    }

    fn rewind(&self, dur_millis: u64) {
        let cursor = {
            let old = self.playback_context.progress_ms.load(Ordering::SeqCst);
            let new = if old <= dur_millis {
                0
            } else {
                old - dur_millis
            };
            new
        };
        self.playback_context
            .progress_ms
            .store(cursor, Ordering::SeqCst);
        self.replay();
    }

    fn speed(&self, speed: f32) {
        self.set_speed(speed);
        self.replay();
    }

    fn playback_context(&self) -> Arc<PlaybackContext> {
        self.playback_context.clone()
    }
}

#[cfg(test)]
mod light_audio_player_tests {
    use std::io::Read;
    use std::sync::atomic::Ordering;
    use std::{fs::File, thread, time::Duration};

    use bytes::Bytes;

    use super::{AudioPlayer, LAudioPlayer};

    fn new() -> (LAudioPlayer, Bytes) {
        let p = LAudioPlayer::try_new().unwrap();
        let mut file = File::open("test-data/bfs.mp3").unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        let track = Bytes::from(buf);
        (p, track)
    }

    #[test]
    fn test_play() {
        let (mut p, track) = new();
        p.play(track).unwrap();
        p.sleep_until_end();
    }

    #[test]
    fn test_volume() {
        let p = LAudioPlayer::try_new().unwrap();
        p.increase_volume(10);
        p.decrease_volume(10);

        assert_eq!(p.volume(), 1.0);
    }

    #[test]
    fn test_pause() {
        let (mut p, track) = new();
        p.play(track).unwrap();

        for i in [0, 1, 2, 3] {
            thread::sleep(Duration::from_secs(5));
            if i % 2 == 0 {
                p.pause();
            } else {
                p.resume();
            }
        }

        println!("must be passed about 4/2 * 5 = 10 seconds");
        println!(
            "timeline cursor: {}s",
            p.playback_context.progress_ms.load(Ordering::Relaxed) / 1000
        );
    }

    #[test]
    fn test_drain_sink() {
        let (mut p, track) = new();
        p.play(track).unwrap();
        thread::sleep(Duration::from_secs(5));
        p.drain_sink();
        p.sleep_until_end();
    }

    #[test]
    fn test_reload() {
        let (mut p, track) = new();
        p.play(track).unwrap();
        thread::sleep(Duration::from_secs(5));
        p.drain_sink();
        thread::sleep(Duration::from_secs(3));

        p.replay();
        thread::sleep(Duration::from_secs(5));
    }

    // do_skip_duration will resolve to infinite loop when skip duration too large
    #[test]
    fn test_forward() {
        let (mut p, track) = new();
        p.play(track).unwrap();

        for _ in [0; 5] {
            thread::sleep(Duration::from_secs(5));
            println!("fast forward 50 seconds");
            p.forward(50 * 1000);
        }
        p.sleep_until_end();
    }

    #[test]
    fn test_rewind() {
        let (mut p, track) = new();
        p.play(track).unwrap();

        for _ in [0; 3] {
            thread::sleep(Duration::from_secs(10));
            println!("rewind 5 seconds");
            p.rewind(5 * 1000);
        }
        thread::sleep(Duration::from_secs(10));
    }

    #[test]
    fn test_speed() {
        let (mut p, track) = new();
        p.play(track).unwrap();

        for speed in [1.0, 1.25, 1.5, 1.75, 2.0, 1.75, 1.5, 1.25, 1.0] {
            thread::sleep(Duration::from_secs(10));
            println!("speed: X{}", speed);
            p.speed(speed);
        }
        p.sleep_until_end();
    }
}
