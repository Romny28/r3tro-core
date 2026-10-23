use std::num::NonZero;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow};
use rodio::Source;
use rodio::mixer::{Mixer, MixerSource};
use wasapi::{
    AudioClient, DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat,
    calculate_period_100ns, initialize_mta,
};

const CHANNELS: usize = 2;
const LAYOUTS: [(usize, usize); 4] = [(32, 32), (32, 24), (24, 24), (16, 16)];
const READY_TIMEOUT: Duration = Duration::from_secs(5);

pub struct Exclusive {
    mixer: Mixer,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Exclusive {
    pub fn open(rate: u32, failed: Arc<AtomicBool>) -> Result<Self> {
        let rate_nz = NonZero::new(rate).context("the sample rate is zero")?;
        let channels = NonZero::new(CHANNELS as u16).context("no channels")?;
        let (mixer, source) = rodio::mixer::mixer(channels, rate_nz);
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::channel::<Result<String, String>>();

        let thread = {
            let stop = stop.clone();
            thread::Builder::new()
                .name("exclusive-output".into())
                .spawn(move || {
                    if let Err(error) = session(rate, source, &stop, &ready_tx) {
                        log::warn!("exclusive: output stopped: {error:#}");
                        ready_tx.send(Err(format!("{error:#}"))).ok();
                        failed.store(true, Ordering::Release);
                    }
                })
                .context("cannot start the exclusive output thread")?
        };

        let this = Self {
            mixer,
            stop,
            thread: Some(thread),
        };
        match ready_rx.recv_timeout(READY_TIMEOUT) {
            Ok(Ok(summary)) => {
                log::info!("exclusive: {summary}");
                Ok(this)
            }
            Ok(Err(message)) => Err(anyhow!(message)),
            Err(_) => Err(anyhow!("the exclusive output did not start in time")),
        }
    }

    pub fn add(&self, source: impl Source + Send + 'static) {
        self.mixer.add(source);
    }
}

impl Drop for Exclusive {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            thread.join().ok();
        }
    }
}

fn wasapi_error(error: impl std::fmt::Display) -> anyhow::Error {
    anyhow!("{error}")
}

fn negotiate(client: &mut AudioClient, rate: u32) -> Result<(usize, usize, WaveFormat)> {
    for (store, valid) in LAYOUTS {
        let wanted = WaveFormat::new(
            store,
            valid,
            &SampleType::Int,
            rate as usize,
            CHANNELS,
            None,
        );
        if let Ok(found) = client.is_supported_exclusive_with_quirks(&wanted)
            && found.get_blockalign() as usize == store / 8 * CHANNELS
            && found.get_samplespersec() as usize == rate as usize
        {
            return Ok((store, valid, found));
        }
    }
    Err(anyhow!("the device accepts no exclusive format at {rate} Hz"))
}

fn encode(out: &mut [u8], sample: f32, store: usize, valid: usize) {
    let scale = (1i64 << (valid - 1)) as f64;
    let top = (1i64 << (valid - 1)) - 1;
    let value = ((f64::from(sample) * scale).round() as i64).clamp(-top - 1, top) << (store - valid);
    out.copy_from_slice(&value.to_le_bytes()[..store / 8]);
}

fn session(
    rate: u32,
    mut source: MixerSource,
    stop: &AtomicBool,
    ready: &mpsc::Sender<Result<String, String>>,
) -> Result<()> {
    let _ = initialize_mta();

    let enumerator = DeviceEnumerator::new().map_err(wasapi_error)?;
    let device = enumerator
        .get_default_device(&Direction::Render)
        .map_err(wasapi_error)?;
    let mut client = device.get_iaudioclient().map_err(wasapi_error)?;

    let (store, valid, format) = negotiate(&mut client, rate)?;
    let blockalign = format.get_blockalign() as usize;
    let store_bytes = store / 8;

    let (default_period, _) = client.get_device_period().map_err(wasapi_error)?;
    let period = client
        .calculate_aligned_period_near(default_period, Some(128), &format)
        .map_err(wasapi_error)?;
    let mode = StreamMode::PollingExclusive {
        period_hns: period,
        buffer_duration_hns: 16 * period,
    };
    if let Err(first) = client.initialize_client(&format, &Direction::Render, &mode) {
        let frames = client
            .get_buffer_size()
            .map_err(|_| wasapi_error(&first))?;
        let aligned = calculate_period_100ns(i64::from(frames), rate as i64);
        client = device.get_iaudioclient().map_err(wasapi_error)?;
        let retry = StreamMode::PollingExclusive {
            period_hns: aligned,
            buffer_duration_hns: 16 * aligned,
        };
        client
            .initialize_client(&format, &Direction::Render, &retry)
            .map_err(|second| anyhow!("{first}; then {second}"))?;
    }

    let render = client.get_audiorenderclient().map_err(wasapi_error)?;
    let buffer_frames = client.get_buffer_size().map_err(wasapi_error)? as usize;
    let sleep = Duration::from_micros(500_000 * buffer_frames as u64 / u64::from(rate));
    let mut bytes = vec![0u8; buffer_frames * blockalign];

    let mut fill = |frames: usize| -> Result<()> {
        let used = (frames * blockalign).min(bytes.len());
        for sample in bytes[..used].chunks_exact_mut(store_bytes) {
            encode(sample, source.next().unwrap_or(0.0), store, valid);
        }
        render
            .write_to_device(used / blockalign, &bytes[..used], None)
            .map_err(wasapi_error)
    };

    let free = client.get_available_space_in_frames().map_err(wasapi_error)? as usize;
    fill(free)?;
    client.start_stream().map_err(wasapi_error)?;
    ready
        .send(Ok(format!(
            "{rate} Hz, {valid} of {store} bits, buffer {buffer_frames} frames"
        )))
        .ok();

    while !stop.load(Ordering::Acquire) {
        thread::sleep(sleep);
        let free = client.get_available_space_in_frames().map_err(wasapi_error)? as usize;
        if free > 0 {
            fill(free)?;
        }
    }
    client.stop_stream().ok();
    Ok(())
}
