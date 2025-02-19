use std::{env, error::Error};

use redlight::{config::CacheConfig, RedisCache};
use twilight_gateway::Event;
use twilight_model::{
    channel::{stage_instance::PrivacyLevel, StageInstance},
    gateway::payload::incoming::StageInstanceCreate,
    id::Id,
};

use self::{config::Config, recorder::PrintRecorder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize env variables from our .env file
    dotenvy::dotenv().unwrap();

    // Set the global metrics recorder. The recorder may originate from
    // dependencies such as https://crates.io/crates/metrics-exporter-prometheus
    // or a custom implementation. Checkout the documentation of the `metrics`
    // crate for more info.
    metrics::set_global_recorder(PrintRecorder)?;

    // Create our cache by using our simple `Config` which only caches stage
    // instances
    let url = env::var("REDIS_URL")?;
    println!("Creating cache...");
    let cache = RedisCache::<Config>::new(&url).await?;

    // Let's create a `StageInstanceCreate` event and update our cache with it
    let stage = StageInstance {
        id: Id::new(1),
        channel_id: Id::new(100),
        guild_id: Id::new(200),
        guild_scheduled_event_id: Default::default(),
        privacy_level: PrivacyLevel::GuildOnly,
        topic: Default::default(),
    };

    let event = Event::StageInstanceCreate(StageInstanceCreate(stage.clone()));
    println!("Updating cache...");
    cache.update(&event).await?;

    // Await the metrics recorder to print its findings
    println!("Sleeping...");
    tokio::time::sleep(Config::METRICS_INTERVAL_DURATION).await;

    // And another one
    let stage = StageInstance {
        id: Id::new(2),
        ..stage.clone()
    };

    let event = Event::StageInstanceCreate(StageInstanceCreate(stage));
    println!("Updating cache...");
    cache.update(&event).await?;

    // Await the metrics recorder again...
    println!("Sleeping...");
    tokio::time::sleep(Config::METRICS_INTERVAL_DURATION).await;

    Ok(())
}

/// Very simple custom metrics recorder.
/// All it does is print out whenever something happens with gauges.
/// Chances are you want to use a dependency instead of a custom implementation.
mod recorder {
    use std::sync::Arc;

    use metrics::{
        Counter, Gauge, GaugeFn, Histogram, Key, KeyName, Metadata, Recorder, SharedString, Unit,
    };

    struct PrintHandle(Key);

    impl GaugeFn for PrintHandle {
        fn set(&self, value: f64) {
            println!("Gauge set for `{}`: {}", self.0.name(), value);
        }

        fn increment(&self, _: f64) {
            unimplemented!()
        }

        fn decrement(&self, _: f64) {
            unimplemented!()
        }
    }

    pub struct PrintRecorder;

    impl Recorder for PrintRecorder {
        fn describe_gauge(&self, key: KeyName, _: Option<Unit>, description: SharedString) {
            println!(
                "Registered key {} with description {description:?}",
                key.as_str()
            );
        }

        fn register_gauge(&self, key: &Key, _: &Metadata) -> Gauge {
            Gauge::from_arc(Arc::new(PrintHandle(key.clone())))
        }

        fn describe_counter(&self, _: KeyName, _: Option<Unit>, _: SharedString) {
            unimplemented!()
        }

        fn describe_histogram(&self, _: KeyName, _: Option<Unit>, _: SharedString) {
            unimplemented!()
        }

        fn register_counter(&self, _: &Key, _: &Metadata) -> Counter {
            unimplemented!()
        }

        fn register_histogram(&self, _: &Key, _: &Metadata) -> Histogram {
            unimplemented!()
        }
    }
}

/// Very simple `CacheConfig` implementation to only cache stage instances
/// and update metrics every 15 seconds.
mod config {
    use std::time::Duration;

    use redlight::config::{CacheConfig, Cacheable, ICachedStageInstance, Ignore};
    use rkyv::{rancor::Source, Archive, Serialize};
    use twilight_model::channel::StageInstance;

    pub struct Config;

    impl CacheConfig for Config {
        const METRICS_INTERVAL_DURATION: Duration = Duration::from_secs(15);

        type Channel<'a> = Ignore;
        type CurrentUser<'a> = Ignore;
        type Emoji<'a> = Ignore;
        type Guild<'a> = Ignore;
        type Integration<'a> = Ignore;
        type Member<'a> = Ignore;
        type Message<'a> = Ignore;
        type Presence<'a> = Ignore;
        type Role<'a> = Ignore;
        type StageInstance<'a> = CachedStageInstance; // <-
        type Sticker<'a> = Ignore;
        type User<'a> = Ignore;
        type VoiceState<'a> = Ignore;
    }

    #[derive(Archive, Serialize)]
    pub struct CachedStageInstance;

    impl<'a> ICachedStageInstance<'a> for CachedStageInstance {
        fn from_stage_instance(_: &'a StageInstance) -> Self {
            Self
        }
    }

    impl Cacheable for CachedStageInstance {
        type Bytes = [u8; 0];

        fn expire() -> Option<Duration> {
            None
        }

        fn serialize_one<E: Source>(&self) -> Result<Self::Bytes, E> {
            Ok([])
        }
    }
}
