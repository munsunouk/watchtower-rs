/// Macro for notification parameter processing
#[macro_export]
macro_rules! process_notification_params {
    ($param_nessesary:expr, $function_params:expr, $notification:expr, $param_config:expr, $context:expr) => {
        for (param_nessery, function_param) in $param_nessesary.iter().zip($function_params.iter())
        {
            match param_nessery.as_str() {
                $crate::utils::constants::PARAM_CHANNEL => {
                    if let Some(GeneralToken::String(channel)) = function_param {
                        for channel_config in $param_config.channel_config.iter() {
                            if (channel_config.name == *channel)
                                && ($notification == $crate::utils::constants::NOTIFICATION_SLACK)
                            {
                                if let Some(client) = &mut $context.slack_client {
                                    client.set_channel(&channel_config.id);
                                }
                            }
                        }
                    }
                }
                $crate::utils::constants::PARAM_TIME_INTERVAL => {
                    if let Some(GeneralToken::Uint(time_interval)) = function_param {
                        if $notification == $crate::utils::constants::NOTIFICATION_SLACK {
                            if let Some(client) = &mut $context.slack_client {
                                client.set_time_interval(time_interval);
                            }
                        }
                    }
                }
                $crate::utils::constants::PARAM_MESSAGE => {
                    if let Some(GeneralToken::String(message)) = function_param {
                        if $notification == $crate::utils::constants::NOTIFICATION_SLACK {
                            if let Some(client) = &mut $context.slack_client {
                                let runtime = tokio::runtime::Runtime::new()?;
                                runtime.block_on(async { client.send_message(&message).await })?;
                            }
                            break;
                        }
                    }
                }
                _ => {
                    return Err(WorkerError::InvalidTypeConvertError(format!(
                        "notification parameter, {}",
                        param_nessery.to_string()
                    )))
                }
            }
        }
    };
}

/// Macro for chain function parameter processing
#[macro_export]
macro_rules! process_chain_function_params {
    ($param_nessesary:expr, $function_params:expr, $context:expr, $blockchain:expr, $target_block_number:expr, $address:expr) => {
        for (param_nessery, function_param) in $param_nessesary.iter().zip($function_params.iter())
        {
            match param_nessery.as_str() {
                $crate::utils::constants::PARAM_BLOCK_NUMBER => {
                    if let Some(token) = function_param {
                        if token.type_check(&ParamType::Uint(256)) {
                            *$target_block_number = token.into_uint()?;
                        }
                    }
                }
                $crate::utils::constants::PARAM_BALANCE => {
                    if let Some(GeneralToken::String(balance_name)) = function_param {
                        for balance in $context.param_config.balance_config.iter() {
                            if (balance.name == *balance_name)
                                && (balance.blockchain == *$blockchain)
                            {
                                *$address = Some(&balance.address);
                                break;
                            }
                        }
                    }
                }
                _ => {
                    return Err(WorkerError::InvalidTypeConvertError(format!(
                        "chain_function parameter, {}",
                        param_nessery.to_string()
                    )))
                }
            }
        }
    };
}

/// Macro for RPC function parameter processing
#[macro_export]
macro_rules! process_rpc_function_params {
    ($param_nessesary:expr, $function_params:expr, $context:expr, $url:expr, $url_token:expr, $api_query:expr, $target_index:expr) => {


        for (param_nessery, function_param) in $param_nessesary.iter().zip($function_params.iter()) {


            match param_nessery.as_str() {
                $crate::utils::constants::PARAM_URL => {

                    if let Some(GeneralToken::String(url_name)) = function_param {

                        for url_config in $context.param_config.url_config.iter() {

                            if url_config.name == *url_name {

                                *$url = Some(&url_config.url);
                                if let Some(token_str) = &url_config.token {
                                    *$url_token = Some(token_str);
                                }
                                break;
                            }
                        }
                    } else {
                        return Err(WorkerError::InvalidTypeConvertError(format!("PARAM_URL function_param is not a String: {:?}", function_param)));
                    }
                },
                $crate::utils::constants::PARAM_KEY => {
                    if let Some(GeneralToken::String(key)) = function_param {
                        *$url_token = Some(key);
                    }
                },

                $crate::utils::constants::PARAM_FEED => {
                    if let Some(GeneralToken::String(target_str)) = function_param {
                        for feed_config in $context.param_config.feed_config.iter() {
                            let FeedConfig {
                                name,
                                target_index: feed_target_index,
                                ..
                            } = feed_config;
                            if name == target_str {
                                *$target_index = format!("{}.{}", $target_index, feed_target_index);
                                break;
                            }
                        }
                    }
                },
                $crate::utils::constants::PARAM_VAULT_ADDRESS => {
                    if let Some(token) = function_param {
                        if token.type_check(&ParamType::Array(Box::new(ParamType::String))) {
                            if let GeneralToken::Array(tokens) = token {
                                let strings: Result<Vec<&str>, WorkerError> = tokens
                                    .iter()
                                    .map(|t| match t {
                                        GeneralToken::String(s) => Ok(s.as_str()),
                                        _ => Err(WorkerError::InvalidTypeConvertError(
                                            format!("Expected String, got {:?}", t),
                                        )),
                                    })
                                    .collect();
                                let joined_string = strings?.join("|");
                                *$api_query = Some(json!({
                                    $crate::utils::constants::API_QUERY_ACTIVE: joined_string
                                }));
                            }
                        } else if token.type_check(&ParamType::String) {
                            if let GeneralToken::String(single_string) = token {
                                *$api_query = Some(json!({
                                    $crate::utils::constants::API_QUERY_ACTIVE: single_string
                                }));
                            }
                        } else if token.type_check(&ParamType::Address) {
                            if let GeneralToken::Address(address) = token {
                                let address_str = format!("{:#x}", address);
                                *$api_query = Some(json!({
                                    $crate::utils::constants::API_QUERY_ACTIVE: address_str
                                }));
                            }
                        } else {
                            return Err(WorkerError::InvalidTypeConvertError(
                                format!("VaultAddress parameter expects String, Array<String>, or Address, got {:?}", token),
                            ));
                        }
                    } else {
                        return Err(WorkerError::InvalidTypeConvertError(
                            "VaultAddress parameter is None".to_string(),
                        ));
                    }
                },
                _ => {
                    return Err(WorkerError::InvalidTypeConvertError(
                        format!("rpc_function parameter, {}", param_nessery.to_string()),
                    ))
                }
            }
        }
    };
}

/// Macro for contract method parameter processing
#[macro_export]
macro_rules! process_contract_method_params {
    ($param_nessesary:expr, $function_params:expr, $context:expr, $target_block_number:expr, $params:expr, $available_contract:expr) => {
        for (param_nessery, function_param) in $param_nessesary.iter().zip($function_params.iter())
        {
            match param_nessery.as_str() {
                $crate::utils::constants::PARAM_BLOCK_NUMBER => {
                    if let Some(token) = function_param {
                        if token.type_check(&ParamType::Uint(256)) {
                            *$target_block_number = token.into_uint()?;
                        }
                    }
                }
                $crate::utils::constants::PARAM_POOL => {
                    if let Some(GeneralToken::String(pool_name)) = function_param {
                        for pool in $context.param_config.pool_config.iter() {
                            if pool.name == *pool_name {
                                $params.push(Some(GeneralToken::Address(parse_to_address(
                                    &pool.address,
                                )?)));
                                break;
                            }
                        }
                    }
                }
                $crate::utils::constants::PARAM_OID => {
                    if let Some(GeneralToken::String(oid_name)) = function_param {
                        for oid in $context.param_config.oid_config.iter() {
                            if oid.name == *oid_name {
                                // Convert to bytes32
                                let bytes = hex::decode(&oid.address[2..])?;
                                $params.push(Some(GeneralToken::FixedBytes(bytes)));
                                break;
                            }
                        }
                    }
                }
                $crate::utils::constants::PARAM_VALIDATOR => {
                    if let Some(GeneralToken::String(validator_name)) = function_param {
                        for validator in $context.param_config.validator_config.iter() {
                            if validator.name == *validator_name {
                                if $available_contract.as_ref().map(|s| s.as_str())
                                    == Some($crate::utils::constants::CONTRACT_STATE)
                                {
                                    $params.push(Some(GeneralToken::Address(parse_to_address(
                                        &validator.address,
                                    )?)));
                                } else if $available_contract.as_ref().map(|s| s.as_str())
                                    == Some($crate::utils::constants::CONTRACT_CANDIDATE)
                                {
                                    $params.push(Some(GeneralToken::Address(parse_to_address(
                                        &validator.controller_address,
                                    )?)));
                                }
                                break;
                            }
                        }
                    }
                }
                _ => {
                    if let Some(token) = function_param {
                        $params.push(Some(token.to_owned()));
                    }
                }
            }
        }
    };
}

/// Macro for implementing TryFromParseResultType for a type
#[macro_export]
macro_rules! impl_try_from_parse_result_type {
    ($t:ty, $variant:ident) => {
        impl TryFromParseResultType for $t {
            fn try_from_parse_result(val: &ParseResultType) -> Result<Self, WorkerError> {
                if let ParseResultType::$variant(v) = val {
                    Ok(v.to_owned())
                } else {
                    Err(WorkerError::InvalidTypeConvertError(format!(
                        "Expected {:?}, got {:?}",
                        stringify!($variant),
                        val
                    )))
                }
            }
        }
    };
}
