use strum::{Display, EnumMessage};
use watch_tower_lib::utils::types::GeneralToken;

#[derive(Debug, Display, EnumMessage)]
#[repr(u16)]
pub enum TraceLog {
    #[strum(to_string = "[Category : {0:?}], [Rule Name : {1}], [Issue : Success]")]
    Success(String, String) = 3002,
    #[strum(to_string = "[Token : {0:?}]")]
    TokenOutput(GeneralToken) = 3003,
}

impl TraceLog {
    #[allow(dead_code)]
    pub fn trace(&self) {
        let msg = format!("[Log Code : {}] ⛔ {}", self.discriminant(), self);

        tracing::trace!("{}", msg);
    }

    pub fn info(&self) {
        let msg = format!("[Log Code : {}] ✨ {}", self.discriminant(), self);

        tracing::info!("{}", msg);
    }

    pub fn debug(&self) {
        let msg = format!("[Log Code : {}] ⚠️ {}", self.discriminant(), self);

        tracing::debug!("{}", msg);
    }

    #[allow(dead_code)]
    pub fn warn(&self) {
        let msg = format!("[Log Code : {}] ⚠️ {}", self.discriminant(), self);

        tracing::warn!("{}", msg);
    }

    fn discriminant(&self) -> u16 {
        unsafe { *(self as *const Self as *const u16) }
    }
}
