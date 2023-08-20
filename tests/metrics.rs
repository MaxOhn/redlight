#![cfg(feature = "metrics")]

use std::{
    error::Error as StdError,
    fmt::Write,
    ops::DerefMut,
    sync::{Arc, Mutex},
    time::Duration,
};

use metrics::{Counter, Gauge, GaugeFn, Histogram, Key, KeyName, Recorder, SharedString, Unit};
use redis::Cmd;
use rkyv::{ser::serializers::AllocSerializer, Archive, Serialize};
use serial_test::serial;
use twilight_model::{
    channel::{message::Sticker, Channel},
    gateway::{
        event::Event,
        payload::incoming::{ChannelCreate, ChannelPinsUpdate, GuildStickersUpdate},
    },
};
use twilight_redis::{
    config::{CacheConfig, Cacheable, ICachedChannel, ICachedSticker, Ignore},
    CacheError, CachedArchive, RedisCache,
};

use crate::{
    events::{channel::text_channel, sticker::stickers},
    pool,
};

#[cfg(feature = "bb8")]
use bb8_redis::redis;

#[cfg(all(not(feature = "bb8"), feature = "deadpool"))]
use deadpool_redis::redis;

#[tokio::test]
#[serial]
async fn test_metrics() -> Result<(), CacheError> {
    struct Config;

    impl CacheConfig for Config {
        const METRICS_INTERVAL_DURATION: Duration = Duration::from_secs(2);

        type Channel<'a> = CachedChannel;
        type CurrentUser<'a> = Ignore;
        type Emoji<'a> = Ignore;
        type Guild<'a> = Ignore;
        type Integration<'a> = Ignore;
        type Member<'a> = Ignore;
        type Message<'a> = Ignore;
        type Presence<'a> = Ignore;
        type Role<'a> = Ignore;
        type StageInstance<'a> = Ignore;
        type Sticker<'a> = CachedSticker;
        type User<'a> = Ignore;
        type VoiceState<'a> = Ignore;
    }

    #[derive(Archive, Serialize)]
    #[cfg_attr(feature = "validation", archive(check_bytes))]
    struct CachedChannel;

    impl<'a> ICachedChannel<'a> for CachedChannel {
        fn from_channel(_: &'a Channel) -> Self {
            Self
        }

        fn on_pins_update(
        ) -> Option<fn(&mut CachedArchive<Self>, &ChannelPinsUpdate) -> Result<(), Box<dyn StdError>>>
        {
            None
        }
    }

    impl Cacheable for CachedChannel {
        type Serializer = AllocSerializer<0>;

        fn expire_seconds() -> Option<usize> {
            None
        }
    }

    #[derive(Archive, Serialize)]
    #[cfg_attr(feature = "validation", archive(check_bytes))]
    struct CachedSticker;

    impl<'a> ICachedSticker<'a> for CachedSticker {
        fn from_sticker(_: &'a Sticker) -> Self {
            Self
        }
    }

    impl Cacheable for CachedSticker {
        type Serializer = AllocSerializer<0>;

        fn expire_seconds() -> Option<usize> {
            None
        }
    }

    struct GaugeHandle {
        value: Mutex<f64>,
    }

    impl GaugeFn for GaugeHandle {
        fn increment(&self, value: f64) {
            self.set(value);
        }

        fn decrement(&self, value: f64) {
            self.set(value);
        }

        fn set(&self, value: f64) {
            *self.value.lock().unwrap() = value;
        }
    }

    #[derive(Clone, Default)]
    struct MetricRecorder {
        inner: Arc<InnerRecorder>,
    }

    #[derive(Default)]
    struct InnerRecorder {
        gauges: Mutex<Vec<(Key, Arc<GaugeHandle>)>>,
    }

    impl MetricRecorder {
        fn render(&self) -> String {
            let mut res = String::new();
            let gauges = self.inner.gauges.lock().unwrap();

            let mut iter = gauges.iter();
            let last = iter.next_back();

            if let Some((key, gauge)) = last {
                for (key, gauge) in iter {
                    let _ = writeln!(res, "{}: {}", key.name(), gauge.value.lock().unwrap());
                }

                let _ = write!(res, "{}: {}", key.name(), gauge.value.lock().unwrap());
            }

            res
        }
    }

    #[rustfmt::skip]
    impl Recorder for MetricRecorder {
        fn register_gauge(&self, key: &Key) -> Gauge {
            let mut gauges = self.inner.gauges.lock().unwrap();

            let new_gauge = match gauges.iter().find(|(entry, _)| entry == key) {
                Some((_, gauge)) => gauge,
                None => {
                    let gauge = Arc::new(GaugeHandle { value: Mutex::new(0.0) });
                    gauges.push((key.to_owned(), gauge));
                    let (_, new_gauge) = &gauges[gauges.len() - 1];

                    new_gauge
                },
            };

            Gauge::from_arc(Arc::clone(new_gauge))
        }

        fn describe_counter(&self, _: KeyName, _: Option<Unit>, _: SharedString) {}
        fn describe_gauge(&self, _: KeyName, _: Option<Unit>, _: SharedString) {}
        fn describe_histogram(&self, _: KeyName, _: Option<Unit>, _: SharedString) {}
        fn register_counter(&self, _: &Key) -> Counter { unimplemented!() }
        fn register_histogram(&self, _: &Key) -> Histogram { unimplemented!() }
    }

    let recorder = MetricRecorder::default();
    metrics::set_boxed_recorder(Box::new(recorder.clone())).unwrap();

    let pool = pool();

    {
        let mut conn = pool.get().await.map_err(CacheError::GetConnection)?;
        Cmd::new()
            .arg("FLUSHDB")
            .query_async(conn.deref_mut())
            .await?;
    }

    let cache = RedisCache::<Config>::with_pool(pool).await?;

    let create_channel = Event::ChannelCreate(Box::new(ChannelCreate(text_channel())));
    cache.update(&create_channel).await?;

    tokio::time::sleep(Config::METRICS_INTERVAL_DURATION + Duration::from_secs(1)).await;

    assert_eq!(recorder.render(), "channel_count: 1\nsticker_count: 0");

    let stickers = stickers();

    let guild_stickers_update = Event::GuildStickersUpdate(GuildStickersUpdate {
        guild_id: stickers[0].guild_id.unwrap(),
        stickers,
    });

    cache.update(&guild_stickers_update).await?;

    tokio::time::sleep(Config::METRICS_INTERVAL_DURATION + Duration::from_secs(1)).await;

    assert_eq!(recorder.render(), "channel_count: 1\nsticker_count: 2");

    Ok(())
}
