use sentry::ClientInitGuard;
use std::borrow::Cow;

use crate::utils::error::INVALID_SENTRY_CLIENT_PARAMS;

/// Builds a sentry client only when the sentry config exists.
pub fn build_sentry_client(
    dsn: &str,
    environment: Option<Cow<'static, str>>,
) -> anyhow::Result<ClientInitGuard> {
    let sentry = sentry::init((
        dsn,
        sentry::ClientOptions {
            release: sentry::release_name!(),

            // https://docs.sentry.io/platforms/rust/configuration/environments/
            environment: environment.clone(),

            // Enable debug mode when needed
            debug: false,

            // To set a uniform sample rate
            // https://docs.sentry.io/platforms/rust/performance/
            traces_sample_rate: 1.0,

            ..Default::default()
        },
    ));

    if !sentry.is_enabled() || dsn.is_empty() {
        tracing::error!(
            "[{}]-[{:?}] ❗️ {}",
            dsn,
            environment,
            INVALID_SENTRY_CLIENT_PARAMS
        );
    }

    Ok(sentry)
}
