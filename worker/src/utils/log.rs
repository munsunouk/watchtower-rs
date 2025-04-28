use ethers::{
    abi::{ParamType, Token},
    types::U64,
};
use strum::{Display, EnumMessage};
use watch_tower_lib::utils::{types::RuleID, DbRuleType};
#[derive(Debug, Display, EnumMessage)]
#[repr(u16)]
pub enum TraceLog {
    #[strum(
        to_string = "[Rule Type : {0:?}], [Rule ID : {1}], [Issue : Failed to decode token: {2:?}] , [Error : {3}]"
    )]
    FailedDecodeLog(DbRuleType, usize, Token, String) = 2101,
    #[strum(to_string = "[Rule Type : {0:?}], [Rule ID : {1}], [Issue : Rule no longer exists]")]
    RuleNotExist(DbRuleType, i32) = 2102,
    #[strum(to_string = "[Rule Type : {0:?}], [Issue : Failed to sync {1} state]")]
    FailedSyncState(DbRuleType, String) = 2103,
    #[strum(to_string = "[Rule Type : {0:?}], [Issue : {1} Rule has no evaluation rules ]")]
    NoEvaluationRules(DbRuleType, RuleID) = 2104,
    #[strum(
        to_string = "[Rule Type : {0:?}], [Issue : {1} Rule has invalid evaluation rule index for ID: {2}]"
    )]
    InvalidEvaluationRule(DbRuleType, RuleID, usize) = 2105,
    #[strum(
        to_string = "[Rule Type : {0:?}], [Rule ID : {1}], [Evaluation ID : {2}], [Issue : Failed to evaluate rule filter: {3}]"
    )]
    FailedEvaluateFilter(DbRuleType, usize, usize, String) = 2106,
    #[strum(to_string = "[Rule Type : {0:?}], [Issue : Updated {1} shared state]")]
    UpdatedSyncState(DbRuleType, String) = 2107,
    #[strum(
        to_string = "[Rule Type : {0:?}], [Rule ID : {1}], [Issue : Failed to decode param data: {2:?}], [Error : {3}]"
    )]
    FailedDecodeParam(DbRuleType, usize, ParamType, String) = 2108,
    #[strum(
        to_string = "[Rule Type : {0:?}], [Rule ID : {1}], [Evaluation ID : {2}], [Value : {3}]"
    )]
    EvaluationValue(DbRuleType, usize, usize, String) = 2109,
    #[strum(
        to_string = "[Rule Type : {0:?}], [Rule ID : {1}], [Evaluation ID : {2}], [Issue : Failed to parse token: {3}]"
    )]
    FailedParseToken(DbRuleType, usize, usize, String) = 2110,
    #[strum(to_string = "[Issue : Monitoring and updating]")]
    MonitoringAndUpdating = 2111,
    #[strum(to_string = "[Rule Type : {0:?}], [Rule ID : {1}]")]
    FetchedRpcCall(DbRuleType, usize) = 2112,
    #[strum(to_string = "[Rule Type : {0:?}], [Rule ID : {1}], [Block Number : ({2:?} … {3:?})]")]
    FetchedContractCall(DbRuleType, usize, U64, U64) = 2113,
    #[strum(to_string = "[Rule Type : {0:?}], [Chain ID : {1}], [Block Number : ({2:?} … {3:?})]")]
    FetchedContractEvent(DbRuleType, u32, U64, U64) = 2114,
    #[strum(
        to_string = "[Rule Type : {0:?}], [Rule ID : {1}], [Issue : Failed to fetch RPC call], [Error : {2}]"
    )]
    FailedFetchedRpcCall(DbRuleType, usize, String) = 2115,
    #[strum(
        to_string = "[Rule Type : {0:?}], [Rule ID : {1}], [Issue : Failed to fetch Contract Call], [Block Number :({2:?} … {3:?})]], [Error : {4}]"
    )]
    FailedFetchedContractCall(DbRuleType, usize, U64, U64, String) = 2116,
    #[strum(
        to_string = "[Rule Type : {0:?}], [Chain ID : {1}], [Issue : Failed to fetch Contract Event], [Block Number :({2:?} … {3:?})]], [Error : {4}]"
    )]
    FailedFetchedContractEvent(DbRuleType, u32, U64, U64, String) = 2117,
    #[strum(
        to_string = "[Rule Type : {0:?}], [Rule ID : {1}], [Issue : Too many blocks to fetch], [Block Length : {2}]"
    )]
    TooManyBlocksToFetch(DbRuleType, usize, U64) = 2118,
    #[strum(to_string = "[Rule Type : {0:?}], [Rule ID : {1}], [Issue : Health check passed]")]
    HealthCheckPassed(DbRuleType, String) = 2119,
    #[strum(to_string = "[Rule Type : {0:?}], [Rule ID : {1}], [Issue : Stop fetcher]")]
    StopFetcher(DbRuleType, usize) = 2120,
}

impl TraceLog {
    pub fn trace(&self) {
        let msg = format!(
            "[Log Code : {}] ⛔ {}",
            self.discriminant(),
            self.to_string()
        );

        tracing::trace!("{}", msg);
    }

    pub fn info(&self) {
        let msg = format!(
            "[Log Code : {}] ✨ {}",
            self.discriminant(),
            self.to_string()
        );

        tracing::info!("{}", msg);
    }

    pub fn warn(&self) {
        let msg = format!(
            "[Log Code : {}] ⚠️ {}",
            self.discriminant(),
            self.to_string()
        );

        tracing::warn!("{}", msg);
    }

    fn discriminant(&self) -> u16 {
        unsafe { *(self as *const Self as *const u16) }
    }
}
