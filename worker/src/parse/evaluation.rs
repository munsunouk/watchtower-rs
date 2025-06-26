use cron::Schedule;
use ethers::abi::{ParamType, Token};

use ethers::types::U256;
use serde_json::{json, Value};

use tokio::runtime::Handle;
use watch_tower_lib::utils::parse_to_address;

use tokio::time::sleep;
use watch_tower_lib::cli::db::data::RuleData;
use watch_tower_lib::cli::slack::SlackNotifier;
use watch_tower_lib::utils::types::ChainID;
use watch_tower_lib::utils::{
    constants::{
        BOOLEAN_LITERAL_FALSE, BOOLEAN_LITERAL_TRUE, LOGIC_OPERATOR_AND, LOGIC_OPERATOR_OR,
    },
    parse_string_to_uint, parse_to_abi,
};

use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use std::fs;

use std::{collections::HashMap, str::FromStr};

use watch_tower_lib::utils::{arithmetic_token, compare_token};

use crate::rule::decode_meta_data;
use crate::rule::get::{get, get_eth_balance, get_latest_block, get_latest_block_number};
use crate::rule::store::{assign, check_store_value, eval, SymbolTable};
use crate::utils::config::{
    BlockchainTargetValue, Configuration, ContractCallTargetValue, ContractConfig,
    ContractEventTargetValue, EVMProvider, FeedConfig, NotificationCallTargetValue,
    NotificationConfig, ParamConfig, RPCTargetValue,
};
use crate::utils::constants::{CONFIG_PATH, HEALETH_CHECK_INTERVAL};
use crate::utils::error::WorkerError;
use crate::utils::log::TraceLog;
use watch_tower_lib::utils::types::GeneralToken;

use hex;

// Import macros from crate root
use crate::{
    impl_try_from_parse_result_type, option_or_err, process_chain_function_params,
    process_contract_method_params, process_notification_params, process_rpc_function_params,
};

/// # Description
/// This struct represents an evaluation rule.
/// # Fields
/// * `id` - The ID of the rule.
/// * `rule_filter` - The rule filter.
/// * `expected_value` - The expected value.
pub struct Evaluator {
    pub context: Context,
}

impl Evaluator {
    pub fn new(config: &Configuration, param_config: &ParamConfig, rule: RuleData) -> Self {
        let mut slack_client = None;

        for notification_config in &config.notification_config {
            let NotificationConfig { service, key } = notification_config;
            if service == "Slack" {
                slack_client = Some(SlackNotifier::new(key));
            }
        }

        let context = Context {
            config: config.to_owned(),
            param_config: param_config.to_owned(),
            symbol_table: SymbolTable::new(),
            variables: HashMap::new(),
            rule: rule.to_owned(),
            slack_client,
        };

        Self { context }
    }

    pub async fn run(&mut self) {
        loop {
            if let Err(e) = self.wait_until_next_time().await {
                tracing::error!("Failed to wait until next time: {}", e);
                WorkerError::FailedSpawn(self.context.rule.name.to_owned(), e.to_string()).log();
            }

            tokio::select! {
                health_check_result = Self::wait_until_next_health_check() => {
                    if health_check_result.is_ok() {
                        self.health_check();
                    }
                }
                process_result = self.process() => {
                    if let Err(e) = process_result {
                        tracing::error!("Process failed for rule {}: {}", self.context.rule.name, e);
                        WorkerError::FailedTask(self.context.rule.name.to_owned(), e.to_string()).log();
                    } else {
                        tracing::debug!("Process completed successfully for rule: {}", self.context.rule.name);
                    }
                }
            }
        }
    }

    async fn process(&mut self) -> Result<(), WorkerError> {
        let rule = self.context.rule.to_owned();
        let context = self.context.to_owned();
        let mut result = GeneralToken::None;

        let context = tokio::task::spawn_blocking(move || {
            let pairs = RuleEvaluationParser::parse(Rule::Program, &rule.script)?;

            let mut context = context;

            for pair in pairs {
                result = parse_pair(pair, &mut context)?;
            }

            TraceLog::TokenOutput(result).debug();

            Ok::<_, WorkerError>(context)
        })
        .await??;

        self.context = context;

        // Only log success if the rule evaluation was successful
        // You might want to add additional checks here based on your business logic
        TraceLog::Success(rule.category, rule.name).info();

        Ok(())
    }

    /// Wait until it reaches the next schedule.
    async fn wait_until_next_time(&self) -> Result<(), WorkerError> {
        let sleep_duration =
            option_or_err!(self.schedule()?.upcoming(chrono::Utc).next()) - chrono::Utc::now();

        match sleep_duration.to_std() {
            Ok(sleep_duration) => {
                sleep(sleep_duration).await;
                Ok(())
            }
            Err(_) => Err(WorkerError::InvalidTypeConvertError(
                "Failed to convert duration".to_string(),
            )),
        }
    }

    async fn wait_until_next_health_check() -> Result<(), WorkerError>
    where
        Self: Sized,
    {
        let health_check_schedule = Self::set_schedule(HEALETH_CHECK_INTERVAL as i32)?;

        let sleep_duration =
            option_or_err!(health_check_schedule.upcoming(chrono::Utc).next()) - chrono::Utc::now();

        match sleep_duration.to_std() {
            Ok(sleep_duration) => {
                sleep(sleep_duration).await;
                Ok(())
            }
            Err(_) => Err(WorkerError::InvalidTypeConvertError(
                "Failed to convert duration".to_string(),
            )),
        }
    }

    fn health_check(&self) {
        TraceLog::HealthCheckPassed(
            self.context.rule.category.to_owned(),
            self.context.rule.name.to_owned(),
        )
        .info()
    }

    /// # Description
    /// This function returns the schedule for the fetcher.
    /// # Returns
    /// * `Result<Schedule, WorkerError>` - The schedule.
    fn schedule(&self) -> Result<Schedule, WorkerError> {
        Self::set_schedule(self.context.rule.time_interval)
    }

    /// # Description
    /// This function sets a cron schedule based on the check interval.
    /// # Arguments
    ///
    /// * `check_interval` - The interval in seconds.
    ///
    /// # Returns
    ///
    /// A `Schedule` instance.
    pub fn set_schedule(check_interval: i32) -> Result<Schedule, WorkerError> {
        let format_schedule = format!("*/{check_interval} * * * * *");

        Ok(Schedule::from_str(&format_schedule)?)
    }
}

#[derive(Parser)]
#[grammar = "parse/evaluation.pest"]
pub struct RuleEvaluationParser;

#[derive(Clone)]
pub struct Context {
    pub config: Configuration,
    pub param_config: ParamConfig,
    pub symbol_table: SymbolTable,
    pub variables: HashMap<String, ParseResultType>,
    pub rule: RuleData,
    pub slack_client: Option<SlackNotifier>,
}

// New synchronous parsing function
fn parse_pair<'a>(
    pair: Pair<'a, Rule>,
    context: &mut Context,
) -> Result<GeneralToken, WorkerError> {
    let runtime = Handle::current();

    match pair.as_rule() {
        Rule::Program => {
            let inner = pair.into_inner();
            let mut result = GeneralToken::None;

            for unwrapped_pair in inner {
                result = parse_pair(unwrapped_pair, context)?;
            }
            Ok(result)
        }
        Rule::ExprStmt => {
            let inner = pair.into_inner();
            let mut result = GeneralToken::None;

            for unwrapped_pair in inner {
                result = parse_pair(unwrapped_pair, context)?;
            }

            Ok(result)
        }
        Rule::AssignmentStmt => {
            let mut inner = pair.into_inner();

            let identifer = option_or_err!(inner.next());

            let first = option_or_err!(inner.next());
            let result = parse_pair(first, context)?;

            assign(&mut context.symbol_table, identifer.as_str(), &result);
            context.variables.clear();

            Ok(result)
        }
        Rule::Expr => {
            let mut inner = pair.into_inner();
            let first = option_or_err!(inner.next());
            let mut result = parse_pair(first, context)?;

            while let Some(op) = inner.next() {
                let next = option_or_err!(inner.next());
                let right = parse_pair(next, context)?;

                result = match op.as_str() {
                    LOGIC_OPERATOR_AND => {
                        if result.type_check(&ParamType::Bool) && right.type_check(&ParamType::Bool)
                        {
                            GeneralToken::Bool(
                                option_or_err!(result.into_bool())
                                    && option_or_err!(right.into_bool()),
                            )
                        } else {
                            return Err(WorkerError::InvalidTypeConvertError(format!(
                                "Expected bool, got {result:?}",
                            )));
                        }
                    }
                    LOGIC_OPERATOR_OR => {
                        if result.type_check(&ParamType::Bool) && right.type_check(&ParamType::Bool)
                        {
                            GeneralToken::Bool(
                                option_or_err!(result.into_bool())
                                    || option_or_err!(right.into_bool()),
                            )
                        } else {
                            return Err(WorkerError::InvalidTypeConvertError(format!(
                                "Expected bool, got {result:?}",
                            )));
                        }
                    }
                    _ => return Err(WorkerError::InvalidOperator(op.as_str().to_string())),
                };
            }
            Ok(result)
        }
        Rule::Condition => {
            let mut inner = pair.into_inner();
            let first = option_or_err!(inner.next());

            let result = parse_pair(first, context)?;
            Ok(result)
        }
        Rule::If => {
            let mut inner = pair.into_inner();
            let mut result = GeneralToken::None;

            // Process all if/else if/else conditions in sequence
            while let Some(token) = inner.next() {
                let literal_text = token.as_str();

                if token.as_rule() == Rule::if_liternal {
                    // This is an if or else if clause - parse the condition
                    let condition_pair = option_or_err!(inner.next());
                    let condition = parse_pair(condition_pair, context)?;

                    if condition.into_bool().unwrap_or(false) {
                        // Parse the if/else if branch
                        let action = option_or_err!(inner.next());
                        result = parse_pair(action, context)?;
                        break;
                    } else {
                        // Skip the if/else if branch and continue to next condition
                        inner.next();
                    }
                } else if token.as_rule() == Rule::else_liternal {
                    let action = option_or_err!(inner.next());
                    result = parse_pair(action, context)?;
                    break;
                }
            }

            Ok(result)
        }
        Rule::Operation => {
            let mut inner = pair.into_inner();
            let left = parse_pair(option_or_err!(inner.next()), context)?;

            if let Some(op) = inner.next() {
                let right = parse_pair(option_or_err!(inner.next()), context)?;

                Ok(compare_token(&left, &right, op.as_str())?)
            } else {
                Ok(left)
            }
        }
        Rule::Term => {
            let mut inner = pair.into_inner();
            let mut result = parse_pair(option_or_err!(inner.next()), context)?;

            while let Some(op) = inner.next() {
                let next = option_or_err!(inner.next());
                let right = parse_pair(next, context)?;

                result = arithmetic_token(&result, &right, op.as_str())?;
            }

            Ok(result)
        }
        Rule::Factor => {
            let mut inner = pair.into_inner();
            let first = option_or_err!(inner.next());
            let mut result = parse_pair(first, context)?;

            while let Some(op) = inner.next() {
                let next = option_or_err!(inner.next());
                let right = parse_pair(next, context)?;

                result = arithmetic_token(&result, &right, op.as_str())?;
            }

            Ok(result)
        }
        Rule::Params => {
            let mut inner = pair.into_inner().peekable();
            let mut params_vec = Vec::new();

            if inner.peek().is_some() {
                for unwrapped_pair in inner {
                    let result = parse_pair(unwrapped_pair, context)?;
                    params_vec.push(Some(result));
                }
            }

            context.variables.insert(
                "function_params".to_string(),
                ParseResultType::ArrayParam(params_vec),
            );

            Ok(GeneralToken::None)
        }
        Rule::Primary => {
            let mut inner = pair.into_inner();
            let first = option_or_err!(inner.next());
            parse_pair(first, context)
        }
        Rule::boolean_literal => match pair.as_str() {
            BOOLEAN_LITERAL_TRUE => Ok(GeneralToken::Bool(true)),
            BOOLEAN_LITERAL_FALSE => Ok(GeneralToken::Bool(false)),
            _ => Err(WorkerError::InvalidOperator(pair.as_str().to_string())),
        },
        Rule::Number => {
            let result = parse_string_to_uint(pair.as_str().to_string())?;
            Ok(GeneralToken::Uint(result))
        }
        Rule::StringLiteral => {
            let string = pair.as_str();

            let string = string.replace("'", "");

            Ok(GeneralToken::String(string.to_string()))
        }
        Rule::if_liternal => {
            let string = pair.as_str();
            Ok(GeneralToken::String(string.to_string()))
        }
        Rule::else_liternal => {
            let string = pair.as_str();
            Ok(GeneralToken::String(string.to_string()))
        }
        Rule::Address => {
            let address = pair.as_str();
            Ok(GeneralToken::Address(parse_to_address(address)?))
        }
        Rule::CallStmt => {
            let inner = pair.into_inner();

            let mut result = GeneralToken::None;

            for unwrapped_pair in inner {
                result = parse_pair(unwrapped_pair, context)?;
            }

            if let Some(_meta_data) = context.variables.get("meta_data") {
                result = decode_meta_data(&result, &mut context.variables)?;
            }

            context.variables.clear();

            Ok(result)
        }
        Rule::NotificationCallExpr => {
            let inner = pair.into_inner();
            let mut result = GeneralToken::None;

            for unwrapped_pair in inner {
                result = parse_pair(unwrapped_pair, context)?;
            }

            let notification: String =
                option_or_err!(context.variables.get("notification")).decode()?;

            let param_nessesary: Vec<String> =
                option_or_err!(context.variables.get("param_nessesary")).decode()?;

            let function_params: Vec<Option<GeneralToken>> =
                option_or_err!(context.variables.get("function_params")).decode()?;

            // Use process_notification_params macro for the cleanest code
            process_notification_params!(
                param_nessesary,
                function_params,
                notification,
                context.param_config,
                context
            );

            Ok(result)
        }
        Rule::ChainFunctionCallExpr => {
            let inner = pair.into_inner();

            for unwrapped_pair in inner {
                parse_pair(unwrapped_pair, context)?;
            }

            let chain_id: i32 = option_or_err!(context.variables.get("chain_id")).decode()?;

            let blockchain: String =
                option_or_err!(context.variables.get("blockchain")).decode()?;

            for provider in context.config.evm_providers.iter() {
                let EVMProvider {
                    name,
                    provider: _,
                    id,
                } = provider;

                if *name == blockchain {
                    context
                        .variables
                        .insert("chain_id".to_string(), ParseResultType::ChainID(*id));
                    context.variables.insert(
                        "blockchain".to_string(),
                        ParseResultType::String(name.to_string()),
                    );
                }
            }

            let param_nessesary: Vec<String> =
                option_or_err!(context.variables.get("param_nessesary")).decode()?;

            let function_params: Vec<Option<GeneralToken>> =
                option_or_err!(context.variables.get("function_params")).decode()?;

            let mut target_block_number = runtime.block_on(async {
                Ok::<U256, WorkerError>(
                    get_latest_block_number(&context.config, chain_id)
                        .await?
                        .into_uint()?,
                )
            })?;

            let mut address: Option<&str> = None;

            process_chain_function_params!(
                param_nessesary,
                function_params,
                context,
                &blockchain,
                &mut target_block_number,
                &mut address
            );

            let name: String = option_or_err!(context.variables.get("name")).decode()?;

            let result: Result<GeneralToken, WorkerError> = match name.as_str() {
                "LatestBlock" => runtime.block_on(async {
                    get_latest_block(&context.config, chain_id, "number".to_string()).await
                }),
                "LatestTimestamp" => runtime.block_on(async {
                    get_latest_block(&context.config, chain_id, "timestamp".to_string()).await
                }),
                "Balance" => runtime.block_on(async {
                    get_eth_balance(
                        &context.config,
                        chain_id,
                        option_or_err!(address),
                        &target_block_number,
                    )
                    .await
                }),
                _ => return Err(WorkerError::InvalidTypeConvertError(name)),
            };

            result
        }
        Rule::RpcFunctionCallExpr => {
            let inner = pair.into_inner();

            for unwrapped_pair in inner {
                parse_pair(unwrapped_pair, context)?;
            }

            let call_type: String = option_or_err!(context.variables.get("call_type")).decode()?;

            let method_type: String =
                option_or_err!(context.variables.get("method_type")).decode()?;

            let param_nessesary: Vec<String> =
                option_or_err!(context.variables.get("param_nessesary")).decode()?;

            let function_params: Vec<Option<GeneralToken>> =
                option_or_err!(context.variables.get("function_params")).decode()?;

            let mut url: Option<&str> = None;
            let mut url_token: Option<&str> = None;

            let api_body =
                if let Some(ParseResultType::Json(api_body)) = context.variables.get("api_body") {
                    Some(api_body.clone())
                } else {
                    None
                };

            let mut api_query = if let Some(ParseResultType::Json(api_query)) =
                context.variables.get("api_query")
            {
                Some(api_query.clone())
            } else {
                None
            };

            let mut target_index: String =
                option_or_err!(context.variables.get("target_index")).decode()?;

            process_rpc_function_params!(
                param_nessesary,
                function_params,
                context,
                &mut url,
                &mut url_token,
                &mut api_query,
                &mut target_index
            );

            let url = option_or_err!(url).to_string();
            let url_token = url_token.map(|s| s.to_string());

            let result = runtime.block_on(async {
                get(
                    &context.config,
                    (
                        url,
                        url_token,
                        call_type,
                        method_type,
                        api_body,
                        api_query,
                        target_index,
                    ),
                )
                .await
            });

            result
        }
        Rule::ContractMethodCallExpr => {
            let inner = pair.into_inner();

            for unwrapped_pair in inner {
                parse_pair(unwrapped_pair, context)?;
            }

            let chain_id: i32 = option_or_err!(context.variables.get("chain_id")).decode()?;
            let address: String = option_or_err!(context.variables.get("address")).decode()?;
            let abi: Value = option_or_err!(context.variables.get("abi")).decode()?;
            let target_index: String =
                option_or_err!(context.variables.get("target_index")).decode()?;

            let mut target_block_number = runtime.block_on(async {
                Ok::<U256, WorkerError>(
                    get_latest_block_number(&context.config, chain_id)
                        .await?
                        .into_uint()?,
                )
            })?;

            let param_nessesary: Vec<String> =
                option_or_err!(context.variables.get("param_nessesary")).decode()?;

            let function_params: Vec<Option<GeneralToken>> =
                option_or_err!(context.variables.get("function_params")).decode()?;

            let mut params: Vec<Option<GeneralToken>> =
                option_or_err!(context.variables.get("method_params")).decode()?;

            let available_contract = if let Some(ParseResultType::String(available_contract)) =
                context.variables.get("available_contract")
            {
                Some(available_contract)
            } else {
                None
            };

            process_contract_method_params!(
                param_nessesary,
                function_params,
                context,
                &mut target_block_number,
                &mut params,
                available_contract
            );

            let result = runtime.block_on(async {
                get(
                    &context.config,
                    (
                        chain_id,
                        address,
                        abi,
                        params,
                        target_index,
                        target_block_number,
                    ),
                )
                .await
            });

            result
        }
        Rule::EventCallExpr => {
            let inner = pair.into_inner();

            for unwrapped_pair in inner {
                parse_pair(unwrapped_pair, context)?;
            }

            let chain_id: i32 = option_or_err!(context.variables.get("chain_id")).decode()?;
            let address: String = option_or_err!(context.variables.get("address")).decode()?;
            let abi: Value = option_or_err!(context.variables.get("abi")).decode()?;
            let event_index: i32 = option_or_err!(context.variables.get("event_index")).decode()?;
            let target_index: String =
                option_or_err!(context.variables.get("target_index")).decode()?;

            let mut target_block_number = runtime.block_on(async {
                Ok::<U256, WorkerError>(
                    get_latest_block_number(&context.config, chain_id)
                        .await?
                        .into_uint()?,
                )
            })?;

            if let Some(ParseResultType::HashMap(identifier)) = context.variables.get("identifier")
            {
                for (key, value) in identifier.iter() {
                    if key.contains("BlockNumber") {
                        if let ParseResultType::Token(token) = value {
                            if token.type_check(&ParamType::Uint(256)) {
                                target_block_number = option_or_err!(token.clone().into_uint());
                            }
                        }
                    }
                }
            }

            let result = runtime.block_on(async {
                get(
                    &context.config,
                    (
                        chain_id,
                        address,
                        abi,
                        event_index,
                        target_index,
                        target_block_number,
                    ),
                )
                .await
            });

            result
        }
        Rule::Chain => {
            let blockchain = pair.as_str();

            for provider in context.config.evm_providers.iter() {
                let EVMProvider {
                    name,
                    provider: _,
                    id,
                } = provider;

                if name == blockchain {
                    context
                        .variables
                        .insert("chain_id".to_string(), ParseResultType::ChainID(*id));
                    context.variables.insert(
                        "blockchain".to_string(),
                        ParseResultType::String(name.to_string()),
                    );
                }
            }

            Ok(GeneralToken::None)
        }
        Rule::Service => {
            let service = pair.as_str();

            if context.variables.contains_key("chain_id") {
                for contract_config in context.config.contract_config.iter() {
                    let ContractConfig {
                        service: parsed_service,
                        blockchain,
                        ..
                    } = contract_config;

                    if service == parsed_service
                        && *blockchain
                            == *option_or_err!(context.variables.get("blockchain"))
                                .decode::<String>()?
                    {
                        context.variables.insert(
                            "service".to_string(),
                            ParseResultType::String(service.to_string()),
                        );
                    }
                }
            }

            Ok(GeneralToken::None)
        }
        Rule::Notification => {
            let notification = pair.as_str();

            for notification_config in context.config.notification_config.iter() {
                let NotificationConfig { service, key } = notification_config;

                if notification == service {
                    context.variables.insert(
                        "notification".to_string(),
                        ParseResultType::String(service.to_string()),
                    );

                    context
                        .variables
                        .insert("key".to_string(), ParseResultType::String(key.to_string()));
                }
            }

            Ok(GeneralToken::None)
        }
        Rule::Contract => {
            let contract_str = pair.as_str();
            let mut found = false;

            for contract_config in context.config.contract_config.iter() {
                let ContractConfig {
                    service: parsed_service,
                    blockchain: parsed_blockchain,
                    contract,
                    address,
                    path,
                    ..
                } = contract_config;

                if let (
                    Some(ParseResultType::String(service)),
                    Some(ParseResultType::String(blockchain)),
                ) = (
                    context.variables.get("service"),
                    context.variables.get("blockchain"),
                ) {
                    if service == parsed_service
                        && contract_str == contract
                        && blockchain == parsed_blockchain
                    {
                        // Get the config file's directory
                        let config_path = option_or_err!(context.config.path.clone());
                        let config_dir =
                            option_or_err!(std::path::Path::new(&config_path).parent());
                        // Resolve the ABI path relative to the config file
                        let abi_path = config_dir.join(path);
                        let abi_content = fs::read_to_string(&abi_path).map_err(|e| {
                            WorkerError::InvalidFileOpenError(format!(
                                "Failed to read ABI file: {}, {}",
                                e,
                                abi_path.display()
                            ))
                        })?;

                        let abi = parse_abi_text(&abi_content)?;

                        context.variables.insert(
                            "contract".to_string(),
                            ParseResultType::String(contract.to_string()),
                        );

                        context
                            .variables
                            .insert("abi".to_string(), ParseResultType::Json(abi));

                        context.variables.insert(
                            "address".to_string(),
                            ParseResultType::String(address.to_string()),
                        );

                        found = false;
                        break;
                    }
                }
            }

            Ok(GeneralToken::Bool(found))
        }
        Rule::Identifier => {
            let identifier = pair.as_str();

            let check_store_value = check_store_value(&context.symbol_table, identifier);

            let result = if check_store_value == GeneralToken::Bool(true) {
                eval(&context.symbol_table, identifier)
            } else {
                GeneralToken::String(identifier.to_string())
            };

            let result_for_hashmap = ParseResultType::GeneralToken(result.clone());

            if let Some(ParseResultType::HashMap(existing)) =
                context.variables.get_mut("identifier")
            {
                existing.insert(identifier.to_string(), result_for_hashmap);
            } else {
                let mut existing = HashMap::new();
                existing.insert(identifier.to_string(), result_for_hashmap);
                context
                    .variables
                    .insert("identifier".to_string(), ParseResultType::HashMap(existing));
            }

            Ok(result)
        }
        Rule::RpcFunctionName => {
            let rpc_call_target_str = pair.as_str();

            for rpc_call_target in context.config.rpc_call_target.iter() {
                let RPCTargetValue {
                    name,
                    meta_data,
                    call_type,
                    method_type,
                    api_body,
                    api_query,
                    target_index,
                    param_nessesary,
                } = rpc_call_target;

                if rpc_call_target_str == name {
                    let mut value_api_body: Option<Value> = None;
                    let mut value_api_query: Option<Value> = None;

                    if let Some(api_body) = api_body {
                        value_api_body = Some(serde_json::from_str(api_body)?);
                    }

                    if let Some(api_query) = api_query {
                        value_api_query = Some(serde_json::from_str(api_query)?);
                    }

                    context.variables.insert(
                        "call_type".to_string(),
                        ParseResultType::String(call_type.to_string()),
                    );
                    context.variables.insert(
                        "method_type".to_string(),
                        ParseResultType::String(method_type.to_string()),
                    );

                    if let Some(value_api_body) = value_api_body {
                        context.variables.insert(
                            "api_body".to_string(),
                            ParseResultType::Json(value_api_body),
                        );
                    }

                    if let Some(value_api_query) = value_api_query {
                        context.variables.insert(
                            "api_query".to_string(),
                            ParseResultType::Json(value_api_query),
                        );
                    }

                    context.variables.insert(
                        "target_index".to_string(),
                        ParseResultType::String(target_index.to_string()),
                    );

                    context.variables.insert(
                        "meta_data".to_string(),
                        ParseResultType::String(meta_data.to_string()),
                    );

                    context.variables.insert(
                        "param_nessesary".to_string(),
                        ParseResultType::Array(param_nessesary.to_vec()),
                    );
                }
            }

            Ok(GeneralToken::None)
        }
        Rule::ContractMethodName => {
            let contract_call_target_str = pair.as_str();

            for contract_call_target in context.config.contract_call_target.iter() {
                let ContractCallTargetValue {
                    name,
                    params,
                    target_index,
                    param_nessesary,
                    available_contract,
                } = contract_call_target;

                if contract_call_target_str == name {
                    let should_insert = if let Some(available_contract) = available_contract {
                        if let Some(ParseResultType::String(contract)) =
                            context.variables.get("contract")
                        {
                            if available_contract == contract {
                                context.variables.insert(
                                    "available_contract".to_string(),
                                    ParseResultType::String(available_contract.to_string()),
                                );
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        true
                    };

                    if should_insert {
                        context.variables.insert(
                            "method_params".to_string(),
                            ParseResultType::ArrayParam(params.to_vec()),
                        );
                        context.variables.insert(
                            "target_index".to_string(),
                            ParseResultType::String(target_index.to_string()),
                        );
                        context.variables.insert(
                            "param_nessesary".to_string(),
                            ParseResultType::Array(param_nessesary.to_vec()),
                        );

                        break;
                    }
                }
            }

            Ok(GeneralToken::None)
        }
        Rule::EventName => {
            let contract_event_target_str = pair.as_str();

            for contract_event_target in context.config.contract_event_target.iter() {
                let ContractEventTargetValue {
                    name,
                    event_index,
                    target_index,
                } = contract_event_target;

                if contract_event_target_str == name {
                    context.variables.insert(
                        "event_index".to_string(),
                        ParseResultType::EventIndex(*event_index),
                    );
                    context.variables.insert(
                        "target_index".to_string(),
                        ParseResultType::String(target_index.to_string()),
                    );
                }
            }

            Ok(GeneralToken::None)
        }
        Rule::NotificationFunctionName => {
            let notification_call_target_str = pair.as_str();

            for notification_call_target in context.config.notification_call_target.iter() {
                let NotificationCallTargetValue {
                    name,
                    params,
                    param_nessesary,
                } = notification_call_target;

                if notification_call_target_str == name {
                    context.variables.insert(
                        "name".to_string(),
                        ParseResultType::ArrayParam(params.to_vec()),
                    );

                    context.variables.insert(
                        "param_nessesary".to_string(),
                        ParseResultType::Array(param_nessesary.to_vec()),
                    );
                }
            }

            Ok(GeneralToken::None)
        }
        Rule::ChainFunctionName => {
            let blockchain_call_target_str = pair.as_str();

            for blockchain_call_target in context.config.blockchain_call_target.iter() {
                let BlockchainTargetValue {
                    name,
                    param_nessesary,
                    ..
                } = blockchain_call_target;

                if blockchain_call_target_str == name {
                    context.variables.insert(
                        "name".to_string(),
                        ParseResultType::String(name.to_string()),
                    );

                    context.variables.insert(
                        "param_nessesary".to_string(),
                        ParseResultType::Array(param_nessesary.to_vec()),
                    );
                }
            }

            Ok(GeneralToken::Bool(false))
        }
        _ => Err(WorkerError::InvalidRuleDecode(format!(
            "Unexpected rule: {:?}",
            pair.as_rule()
        ))),
    }
}

/// CheckFunctionLength is the function to check the function length.
/// # Arguments
/// * `abi_text` - The ABI text.
/// # Returns
/// * `Result<bool, GeneralError>` - The result.
pub fn _check_function_length(abi_text: &str) -> Result<bool, WorkerError> {
    let abi_value = parse_abi_text(abi_text)?;

    let abi = parse_to_abi(abi_value)?;

    let function_count = abi.functions().count();

    if function_count != 1 {
        return Ok(false);
    }
    Ok(true)
}

/// CheckEventLength is the function to check the event length.
/// # Arguments
/// * `abi_text` - The ABI text.
/// # Returns
/// * `Result<bool, GeneralError>` - The result.
pub fn _check_event_length(abi_text: &str) -> Result<bool, WorkerError> {
    let abi_value = parse_abi_text(abi_text)?;

    let abi = parse_to_abi(abi_value)?;

    let event_count = abi.events().count();

    if event_count != 1 {
        return Ok(false);
    }

    Ok(true)
}

/// ParseABIText is the function to parse the ABI text.
/// # Arguments
/// * `abi_text` - The ABI text.
/// # Returns
/// * `Result<Value, GeneralError>` - The result.
pub fn parse_abi_text(abi_text: &str) -> Result<Value, WorkerError> {
    Ok(serde_json::from_str(abi_text)?)
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ParseResultType {
    String(String),
    _Number(U256),
    _Bool(bool),
    Json(Value),
    ChainID(ChainID),
    Array(Vec<String>),
    ArrayParam(Vec<Option<GeneralToken>>),
    GeneralToken(GeneralToken),
    Token(Token),
    EventIndex(i32),
    HashMap(HashMap<String, ParseResultType>),
}

/// Trait for decoding ParseResultType into concrete types
pub trait Decode {
    fn decode<T>(&self) -> Result<T, WorkerError>
    where
        T: TryFromParseResultType;
}

impl Decode for ParseResultType {
    fn decode<T>(&self) -> Result<T, WorkerError>
    where
        T: TryFromParseResultType,
    {
        T::try_from_parse_result(self)
    }
}

/// Helper trait for type-safe conversion from ParseResultType
pub trait TryFromParseResultType: Sized {
    fn try_from_parse_result(val: &ParseResultType) -> Result<Self, WorkerError>;
}

impl_try_from_parse_result_type!(String, String);
impl_try_from_parse_result_type!(bool, _Bool);
impl_try_from_parse_result_type!(U256, _Number);
impl_try_from_parse_result_type!(ChainID, ChainID);

// Add implementations for other variants
impl TryFromParseResultType for Value {
    fn try_from_parse_result(val: &ParseResultType) -> Result<Self, WorkerError> {
        if let ParseResultType::Json(v) = val {
            Ok(v.to_owned())
        } else {
            Err(WorkerError::InvalidTypeConvertError(format!(
                "Expected JSON, got {val:?}",
            )))
        }
    }
}

impl TryFromParseResultType for Vec<String> {
    fn try_from_parse_result(val: &ParseResultType) -> Result<Self, WorkerError> {
        if let ParseResultType::Array(v) = val {
            Ok(v.to_vec())
        } else {
            Err(WorkerError::InvalidTypeConvertError(format!(
                "Expected Array, got {val:?}",
            )))
        }
    }
}

impl TryFromParseResultType for Vec<Option<GeneralToken>> {
    fn try_from_parse_result(val: &ParseResultType) -> Result<Self, WorkerError> {
        if let ParseResultType::ArrayParam(v) = val {
            Ok(v.to_vec())
        } else {
            Err(WorkerError::InvalidTypeConvertError(format!(
                "Expected ArrayParam, got {val:?}",
            )))
        }
    }
}

impl TryFromParseResultType for i32 {
    fn try_from_parse_result(val: &ParseResultType) -> Result<Self, WorkerError> {
        match val {
            ParseResultType::ChainID(id) => Ok(*id as i32),
            ParseResultType::EventIndex(idx) => Ok(*idx),
            _ => Err(WorkerError::InvalidTypeConvertError(format!(
                "Expected ChainID or EventIndex, got {val:?}",
            ))),
        }
    }
}

// Example usage:
// let chain_id: ChainID = parse_result.decode()?;
// let s: String = parse_result.decode()?;

#[cfg(test)]
mod tests {

    use crate::utils::setting::{set_config, set_param_config};

    use super::*;

    // Initialize tracing once for all tests in this module
    fn init_tracing() {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            tracing_subscriber::fmt().with_env_filter("debug").init();
        });
    }

    #[test]
    fn test_new_parse_rule() {
        init_tracing();

        let test_input = "
        kicked_out_status = 2;
  notify_time_interval = 60 * 60 * 3;
  node_name = 'Theori';
  kiked_out_str = 'kicked out';
  idle_str = 'idle';

  bifrostBN = Bifrost.LatestBlock();
  node_status = Bifrost.Validator.Candidate.Status(bifrostBN, node_name);

  msg = '*Node Liveness 알람* 🚀\n> Node : ' + node_name + '\n> 상태 :' + idle_str;

          kicked_out_status;

        a = if node_status != kicked_out_status (
    msg;
) else if node_status != kicked_out_status (
    kiked_out_str;
) else (
    idle_str;
);
        a;
        ";

        let pairs = RuleEvaluationParser::parse(Rule::Program, &test_input).unwrap();
        println!("pairs: {pairs:?}");
    }

    #[tokio::test]
    async fn test_evaluation() {
        init_tracing();

        let test_input = "

  notify_time_interval = 60 * 60 * 3;
  relayer_name = 'Theori';

  bifrostBN = Bifrost.LatestBlock();
  relayer_status = Bifrost.CCCP.State.Status(bifrostBN, relayer_name);

  status_name = if relayer_status == 1 (
    'active';
  ) else if relayer_status == 2 (
    'kicked out';
  ) else if relayer_status == 0 (
    'idle';
  ) else (
    'leaving';
  );

  msg = '*Relayer Liveness 알람* 🚀\n> Relayer : ' + relayer_name + '\n> 상태 :' + status_name;

  if relayer_status == 1 (
    msg;
  );
        ";

        let config_path_str = "/Users/munseon-ug/rust/watchtower/worker/config.yaml";
        let param_config_path_str = "/Users/munseon-ug/rust/watchtower/worker/param.yaml";

        let config_path = std::path::PathBuf::from(config_path_str);
        let param_config_path = std::path::PathBuf::from(param_config_path_str);

        let mut config = set_config(&config_path).unwrap();
        config.set_path(config_path_str);

        let param_config = set_param_config(&param_config_path).unwrap();

        let rule = RuleData {
            category: "test".to_string(),
            name: "test".to_string(),
            time_interval: 1,
            script: test_input.to_string(),
        };

        let mut evaluator = Evaluator::new(&config, &param_config, rule);

        let result = evaluator.process().await;

        tracing::debug!("Evaluation result: {:?}", result);
    }
}
