//! Procedural chiptune audio through `AudioToolbox`'s `AudioQueue`.
//!
//! There are no audio files. A small polyphonic synthesiser generates square,
//! triangle, saw, and noise voices in the audio callback. The game thread only
//! enqueues semantic cues; it never touches the sample buffer directly.

use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

use objc2_audio_toolbox::{
    AudioQueueAllocateBuffer, AudioQueueBufferRef, AudioQueueDispose, AudioQueueEnqueueBuffer,
    AudioQueueNewOutput, AudioQueueRef, AudioQueueSetParameter, AudioQueueStart,
    kAudioQueueParam_Volume,
};
use objc2_core_audio_types::{
    AudioStreamBasicDescription, kAudioFormatLinearPCM, kLinearPCMFormatFlagIsPacked,
    kLinearPCMFormatFlagIsSignedInteger,
};

const SAMPLE_RATE: f32 = 44_100.0;
const VOICES: usize = 16;
const BUFFER_COUNT: usize = 4;
const BUFFER_BYTES: u32 = 8_192;

/// Semantic sounds the app asks for. The synth owns the actual pitches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundCue {
    /// One of the four descending marching notes.
    March(u8),
    /// The player's laser.
    PlayerShoot,
    /// An invader's return fire.
    EnemyShoot,
    /// An invader destroyed.
    InvaderDeath,
    /// The cannon destroyed.
    PlayerDeath,
    /// The mystery ship entering.
    MysteryShip,
    /// The mystery ship destroyed.
    MysteryDeath,
    /// Bonus life.
    ExtraLife,
    /// Wave cleared.
    WaveClear,
    /// Round over.
    GameOver,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Waveform {
    Square,
    Triangle,
    Saw,
    Noise,
}

#[derive(Clone, Copy, Debug)]
struct Voice {
    active: bool,
    waveform: Waveform,
    phase: f32,
    start_hz: f32,
    end_hz: f32,
    elapsed: u32,
    duration: u32,
    delay: u32,
    amplitude: f32,
    attack: u32,
    noise_state: u32,
}

impl Voice {
    const SILENT: Self = Self {
        active: false,
        waveform: Waveform::Square,
        phase: 0.0,
        start_hz: 0.0,
        end_hz: 0.0,
        elapsed: 0,
        duration: 0,
        delay: 0,
        amplitude: 0.0,
        attack: 0,
        noise_state: 0x1234_5678,
    };

    fn trigger(
        &mut self,
        waveform: Waveform,
        start_hz: f32,
        end_hz: f32,
        seconds: f32,
        amplitude: f32,
        delay_seconds: f32,
    ) {
        self.active = true;
        self.waveform = waveform;
        self.phase = 0.0;
        self.start_hz = start_hz;
        self.end_hz = end_hz;
        self.elapsed = 0;
        self.duration = (seconds * SAMPLE_RATE) as u32;
        self.delay = (delay_seconds * SAMPLE_RATE) as u32;
        self.amplitude = amplitude;
        self.attack = (0.004 * SAMPLE_RATE) as u32;
    }

    fn sample(&mut self) -> f32 {
        if !self.active || self.duration == 0 {
            return 0.0;
        }
        if self.delay > 0 {
            self.delay -= 1;
            return 0.0;
        }
        if self.elapsed >= self.duration {
            self.active = false;
            return 0.0;
        }

        let progress = self.elapsed as f32 / self.duration as f32;
        let frequency = self.start_hz + (self.end_hz - self.start_hz) * progress;
        self.phase = (self.phase + frequency / SAMPLE_RATE).fract();
        self.elapsed += 1;

        let attack = if self.attack > 0 {
            (self.elapsed as f32 / self.attack as f32).min(1.0)
        } else {
            1.0
        };
        let release = (1.0 - progress).powf(1.4);
        let envelope = attack * release * self.amplitude;

        let raw = match self.waveform {
            Waveform::Square => {
                if self.phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::Triangle => 1.0 - 4.0 * (self.phase - 0.5).abs(),
            Waveform::Saw => self.phase * 2.0 - 1.0,
            Waveform::Noise => {
                self.noise_state = self
                    .noise_state
                    .wrapping_mul(1_664_525)
                    .wrapping_add(1_013_904_223);
                (self.noise_state >> 8) as f32 / 8_388_607.5 - 1.0
            }
        };
        raw * envelope
    }
}

struct Synth {
    voices: [Voice; VOICES],
    muted: bool,
}

impl Synth {
    fn new() -> Self {
        Self {
            voices: [Voice::SILENT; VOICES],
            muted: false,
        }
    }

    fn next_voice(&mut self) -> &mut Voice {
        if let Some(index) = self.voices.iter().position(|voice| !voice.active) {
            return &mut self.voices[index];
        }
        // All voices are busy: steal the one closest to finishing.
        let index = self
            .voices
            .iter()
            .enumerate()
            .min_by_key(|(_, voice)| voice.duration.saturating_sub(voice.elapsed))
            .map_or(0, |(index, _)| index);
        &mut self.voices[index]
    }

    fn trigger(&mut self, cue: SoundCue) {
        let voice = self.next_voice();
        match cue {
            SoundCue::March(step) => {
                let frequencies = [110.0, 98.0, 87.3, 77.8];
                let frequency = frequencies[usize::from(step) % frequencies.len()];
                voice.trigger(Waveform::Square, frequency, frequency, 0.09, 0.30, 0.0);
            }
            SoundCue::PlayerShoot => {
                voice.trigger(Waveform::Square, 1_180.0, 320.0, 0.13, 0.26, 0.0);
            }
            SoundCue::EnemyShoot => {
                voice.trigger(Waveform::Triangle, 360.0, 140.0, 0.16, 0.24, 0.0);
            }
            SoundCue::InvaderDeath => {
                voice.trigger(Waveform::Noise, 0.0, 0.0, 0.16, 0.30, 0.0);
                let echo = self.next_voice();
                echo.trigger(Waveform::Square, 520.0, 120.0, 0.12, 0.18, 0.03);
            }
            SoundCue::PlayerDeath => {
                voice.trigger(Waveform::Saw, 520.0, 45.0, 0.72, 0.34, 0.0);
                let noise = self.next_voice();
                noise.trigger(Waveform::Noise, 0.0, 0.0, 0.45, 0.20, 0.08);
            }
            SoundCue::MysteryShip => {
                voice.trigger(Waveform::Triangle, 780.0, 1_180.0, 0.42, 0.16, 0.0);
                let wobble = self.next_voice();
                wobble.trigger(Waveform::Square, 1_560.0, 1_240.0, 0.42, 0.08, 0.02);
            }
            SoundCue::MysteryDeath => {
                voice.trigger(Waveform::Noise, 0.0, 0.0, 0.28, 0.28, 0.0);
                let zap = self.next_voice();
                zap.trigger(Waveform::Square, 1_700.0, 180.0, 0.30, 0.20, 0.0);
            }
            SoundCue::ExtraLife => {
                for (index, frequency) in [523.25, 659.25, 783.99, 1_046.5].into_iter().enumerate()
                {
                    let note = self.next_voice();
                    note.trigger(
                        Waveform::Square,
                        frequency,
                        frequency,
                        0.12,
                        0.20,
                        index as f32 * 0.09,
                    );
                }
            }
            SoundCue::WaveClear => {
                for (index, frequency) in [392.0, 523.25, 659.25, 783.99].into_iter().enumerate() {
                    let note = self.next_voice();
                    note.trigger(
                        Waveform::Triangle,
                        frequency,
                        frequency,
                        0.18,
                        0.18,
                        index as f32 * 0.10,
                    );
                }
            }
            SoundCue::GameOver => {
                for (index, frequency) in [392.0, 329.63, 261.63, 196.0].into_iter().enumerate() {
                    let note = self.next_voice();
                    note.trigger(
                        Waveform::Saw,
                        frequency,
                        frequency * 0.98,
                        0.28,
                        0.20,
                        index as f32 * 0.16,
                    );
                }
            }
        }
    }

    fn render(&mut self, output: &mut [i16]) {
        for sample in output {
            let mixed: f32 = self.voices.iter_mut().map(Voice::sample).sum();
            *sample = if self.muted {
                0
            } else {
                (mixed.clamp(-1.0, 1.0) * 0.9 * f32::from(i16::MAX)) as i16
            };
        }
    }
}

unsafe extern "C-unwind" fn audio_callback(
    user_data: *mut c_void,
    queue: AudioQueueRef,
    buffer: AudioQueueBufferRef,
) {
    if user_data.is_null() || queue.is_null() || buffer.is_null() {
        return;
    }
    // SAFETY: `user_data` is the `Arc<Mutex<Synth>>` installed in
    // `AudioEngine::new` and lives until the queue is disposed in `Drop`.
    let shared = unsafe { &*user_data.cast::<Arc<Mutex<Synth>>>() };
    // SAFETY: AudioToolbox hands the callback a valid writable playback buffer.
    let audio_buffer = unsafe { &mut *buffer };
    let byte_capacity = audio_buffer.mAudioDataBytesCapacity as usize;
    let samples = byte_capacity / std::mem::size_of::<i16>();

    let mut scratch = [0_i16; (BUFFER_BYTES as usize) / 2];
    let count = samples.min(scratch.len());
    if let Ok(mut synth) = shared.lock() {
        synth.render(&mut scratch[..count]);
    }
    // SAFETY: the destination has `byte_capacity` bytes and `count` samples fit.
    unsafe {
        std::ptr::copy_nonoverlapping(
            scratch.as_ptr(),
            audio_buffer.mAudioData.as_ptr().cast::<i16>(),
            count,
        );
        std::ptr::write_bytes(
            audio_buffer.mAudioData.as_ptr().cast::<u8>().add(count * 2),
            0,
            byte_capacity.saturating_sub(count * 2),
        );
    }
    audio_buffer.mAudioDataByteSize = (count * 2) as u32;
    // SAFETY: re-enqueueing the buffer from its completion callback is the
    // documented AudioQueue ring-buffer contract.
    unsafe {
        AudioQueueEnqueueBuffer(queue, buffer, 0, std::ptr::null());
    }
}

/// Owns the audio queue and the synthesiser shared with its callback thread.
pub struct AudioEngine {
    queue: AudioQueueRef,
    synth: Arc<Mutex<Synth>>,
    shared: *mut Arc<Mutex<Synth>>,
}

// The queue pointer is only used for lifecycle calls on the main thread; the
// synthesiser itself is protected by a mutex. This app is single-window and
// does not move the engine between threads, but marking it Send makes the
// ownership model explicit for future callers.
// SAFETY: all mutable synthesiser state is behind the `Arc<Mutex<Synth>>`, and
// the raw AudioQueue pointer is only touched from `&self` lifecycle methods.
unsafe impl Send for AudioEngine {}

impl AudioEngine {
    /// Starts a 44.1 kHz mono float synth. Failure is non-fatal: the game runs
    /// muted if the audio device cannot be opened.
    pub fn new() -> Result<Self, String> {
        let format = AudioStreamBasicDescription {
            mSampleRate: f64::from(SAMPLE_RATE),
            mFormatID: kAudioFormatLinearPCM,
            mFormatFlags: kLinearPCMFormatFlagIsSignedInteger | kLinearPCMFormatFlagIsPacked,
            mBytesPerPacket: 2,
            mFramesPerPacket: 1,
            mBytesPerFrame: 2,
            mChannelsPerFrame: 1,
            mBitsPerChannel: 16,
            mReserved: 0,
        };
        let synth = Arc::new(Mutex::new(Synth::new()));
        let shared = Box::into_raw(Box::new(Arc::clone(&synth)));
        let mut queue: AudioQueueRef = std::ptr::null_mut();

        // SAFETY: the format and out-pointer are valid for the call, and the
        // callback user data owns a boxed Arc that outlives the queue.
        let status = unsafe {
            AudioQueueNewOutput(
                NonNull::from(&format),
                Some(audio_callback),
                shared.cast::<c_void>(),
                None,
                None,
                0,
                NonNull::from(&mut queue),
            )
        };
        if status != 0 || queue.is_null() {
            // SAFETY: the queue failed to start, so no callback can observe
            // the user-data pointer; reclaim it exactly once.
            unsafe { drop(Box::from_raw(shared)) };
            return Err(format!("AudioQueueNewOutput failed with OSStatus {status}"));
        }

        // SAFETY: the queue was created above and is still valid.
        unsafe {
            AudioQueueSetParameter(queue, kAudioQueueParam_Volume, 0.45);
        }

        for _ in 0..BUFFER_COUNT {
            let mut buffer: AudioQueueBufferRef = std::ptr::null_mut();
            // SAFETY: the queue and out-pointer are valid; AudioQueue owns the
            // allocated buffer until the queue is disposed.
            let status = unsafe {
                AudioQueueAllocateBuffer(queue, BUFFER_BYTES, NonNull::from(&mut buffer))
            };
            if status != 0 || buffer.is_null() {
                // SAFETY: cleanup of the queue created above, before the
                // callback user data is reclaimed.
                unsafe {
                    AudioQueueDispose(queue, true);
                    drop(Box::from_raw(shared));
                }
                return Err(format!(
                    "AudioQueueAllocateBuffer failed with OSStatus {status}"
                ));
            }
            // SAFETY: the freshly allocated buffer has `BUFFER_BYTES` writable
            // bytes, and re-enqueueing it is the documented AudioQueue setup.
            unsafe {
                let audio_buffer = &mut *buffer;
                std::ptr::write_bytes(
                    audio_buffer.mAudioData.as_ptr().cast::<u8>(),
                    0,
                    BUFFER_BYTES as usize,
                );
                audio_buffer.mAudioDataByteSize = BUFFER_BYTES;
                AudioQueueEnqueueBuffer(queue, buffer, 0, std::ptr::null());
            }
        }

        // SAFETY: the queue is fully configured and has four buffers enqueued.
        let status = unsafe { AudioQueueStart(queue, std::ptr::null()) };
        if status != 0 {
            // SAFETY: the queue never started; dispose it, then reclaim the
            // callback user data that was never observed.
            unsafe {
                AudioQueueDispose(queue, true);
                drop(Box::from_raw(shared));
            }
            return Err(format!("AudioQueueStart failed with OSStatus {status}"));
        }

        Ok(Self {
            queue,
            synth,
            shared,
        })
    }

    /// Queues a semantic sound effect.
    pub fn play(&self, cue: SoundCue) {
        if let Ok(mut synth) = self.synth.lock() {
            synth.trigger(cue);
        }
    }

    /// Mutes or unmutes without tearing down the device.
    pub fn set_muted(&self, muted: bool) {
        if let Ok(mut synth) = self.synth.lock() {
            synth.muted = muted;
        }
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        if !self.queue.is_null() {
            // SAFETY: the queue was created by us and is not used afterwards.
            unsafe {
                let _ = objc2_audio_toolbox::AudioQueueStop(self.queue, true);
                let _ = AudioQueueDispose(self.queue, true);
            }
        }
        if !self.shared.is_null() {
            // SAFETY: `shared` came from `Box::into_raw` in `new` and is
            // reclaimed exactly once here, after the queue is disposed.
            unsafe {
                drop(Box::from_raw(self.shared));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SoundCue, Synth, Voice};

    #[test]
    fn a_voice_eventually_finishes() {
        let mut voice = Voice::SILENT;
        voice.trigger(super::Waveform::Square, 440.0, 220.0, 0.01, 1.0, 0.0);
        let mut samples = Vec::new();
        for _ in 0..1_000 {
            samples.push(voice.sample());
        }
        assert!(samples.iter().any(|sample| *sample != 0.0));
        assert!(!voice.active);
    }

    #[test]
    fn cue_trigger_populates_voices() {
        let mut synth = Synth::new();
        synth.trigger(SoundCue::ExtraLife);
        let active = synth.voices.iter().filter(|voice| voice.active).count();
        assert_eq!(active, 4);
        let mut output = [0_i16; 128];
        synth.render(&mut output);
        assert!(output.iter().any(|sample| *sample != 0));
    }
}
