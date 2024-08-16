use sentry::ClientInitGuard;
use std::borrow::Cow;

use crate::utils::error::SentryError;

/// Builds a sentry client only when the sentry config exists.
pub fn build_sentry_client(
    dsn: &str,
    environment: Option<Cow<'static, str>>,
) -> Result<ClientInitGuard, SentryError> {
    let sentry = sentry::init((
        dsn,
        sentry::ClientOptions {
            release: sentry::release_name!(),

            // https://docs.sentry.io/platforms/rust/configuration/environments/
            environment: environment.clone(),

            // Enable debug mode when needed
            debug: false,

            ..Default::default()
        },
    ));

    if !sentry.is_enabled() || dsn.is_empty() {
        tracing::error!(
            "[{}]-[{:?}] ❗️ {}",
            dsn,
            environment,
            SentryError::InvalidParams
        );
    }

    Ok(sentry)
}
