-- Insert sample data into contract_call_rule
INSERT INTO contract_call_rule (id, name, chain_id, address, abi, method_params, rule_filter, expected_value_index, expected_value, comparator, check_interval, created_at, updated_at)
VALUES 
(1, 'Bifrost_BIFI_USDT-deposit_check', 49088, '0xb871966e866F684681f9F44A69BF19652C0c462c', '[
  {
    "inputs": [],
    "name": "callProxyMarket_getMarket",
    "outputs": [
      {
        "components": [
          {
            "internalType": "uint256",
            "name": "handlerID",
            "type": "uint256"
          },
          {
            "internalType": "address",
            "name": "handlerAddr",
            "type": "address"
          },
          {
            "internalType": "uint256",
            "name": "tokenPrice",
            "type": "uint256"
          },
          {
            "internalType": "uint256",
            "name": "depositTotalAmount",
            "type": "uint256"
          },
          {
            "internalType": "uint256",
            "name": "borrowTotalAmount",
            "type": "uint256"
          },
          {
            "internalType": "uint256",
            "name": "depositInterestRate",
            "type": "uint256"
          },
          {
            "internalType": "uint256",
            "name": "borrowInterestRate",
            "type": "uint256"
          }
        ],
        "internalType": "struct CallProxy.callProxyMarket_HandlerAsset[]",
        "name": "",
        "type": "tuple[]"
      },
      {
        "internalType": "bool",
        "name": "",
        "type": "bool"
      }
    ],
    "stateMutability": "view",
    "type": "function"
  }
]', '{}', '{0.0.0-0}', '0.0.2', '52172529092731248', '<', 15, '2024-08-02', '2024-08-02'),
(2, 'Bifrost_Everdex_USDT-USDC_liqudity_check', 49088, '0xD9d3BA810e6F015d1cE6b69d93dfD6bbA7f3c423', '[
  {
    "type": "function",
    "name": "get_pool_info",
    "stateMutability": "view",
    "inputs": [
      {
        "name": "_pool",
        "type": "address"
      }
    ],
    "outputs": [
      {
        "name": "balances",
        "type": "uint256[8]"
      },
      {
        "name": "underlying_balances",
        "type": "uint256[8]"
      },
      {
        "name": "decimals",
        "type": "uint256[8]"
      },
      {
        "name": "underlying_decimals",
        "type": "uint256[8]"
      },
      {
        "name": "rates",
        "type": "uint256[8]"
      },
      {
        "name": "lp_token",
        "type": "address"
      },
      {
        "name": "params",
        "type": "tuple",
        "components": [
          {
            "name": "A",
            "type": "uint256"
          },
          {
            "name": "future_A",
            "type": "uint256"
          },
          {
            "name": "fee",
            "type": "uint256"
          },
          {
            "name": "admin_fee",
            "type": "uint256"
          },
          {
            "name": "future_fee",
            "type": "uint256"
          },
          {
            "name": "future_admin_fee",
            "type": "uint256"
          },
          {
            "name": "future_owner",
            "type": "address"
          },
          {
            "name": "initial_A",
            "type": "uint256"
          },
          {
            "name": "initial_A_time",
            "type": "uint256"
          },
          {
            "name": "future_A_time",
            "type": "uint256"
          }
        ]
      },
      {
        "name": "is_meta",
        "type": "bool"
      },
      {
        "name": "name",
        "type": "string"
      }
    ]
  }
]
', '{0x8cfcBc421334263ed3A2f62B49Ee7A471Ade7aBb}', '{}', '0.2', '160304504480', '<', 15, '2024-08-02', '2024-08-02');

-- Insert sample data into contract_event_rule
INSERT INTO contract_event_rule (id, name, chain_id, address, abi, event_index, rule_filter, expected_value_index, expected_value, comparator, created_at, updated_at)
VALUES 
(1, 'Bifrost_CCCP_USDC_min_check', 49088, '0x0218371b18340aBD460961bdF3Bd5F01858dAB53', '[
  {
    "anonymous": false,
    "inputs": [
      {
        "components": [
          {
            "components": [
              {
                "internalType": "ChainIndex",
                "name": "chain",
                "type": "bytes4"
              },
              {
                "internalType": "uint64",
                "name": "round_id",
                "type": "uint64"
              },
              {
                "internalType": "uint128",
                "name": "sequence",
                "type": "uint128"
              }
            ],
            "internalType": "struct Socket_Struct.RequestID",
            "name": "req_id",
            "type": "tuple"
          },
          {
            "internalType": "enum Socket_Struct.Task_Status",
            "name": "status",
            "type": "uint8"
          },
          {
            "components": [
              {
                "internalType": "ChainIndex",
                "name": "chain",
                "type": "bytes4"
              },
              {
                "internalType": "RBCmethod",
                "name": "method",
                "type": "bytes16"
              }
            ],
            "internalType": "struct Socket_Struct.Instruction",
            "name": "ins_code",
            "type": "tuple"
          },
          {
            "components": [
              {
                "internalType": "Asset_Index",
                "name": "tokenIDX0",
                "type": "bytes32"
              },
              {
                "internalType": "Asset_Index",
                "name": "tokenIDX1",
                "type": "bytes32"
              },
              {
                "internalType": "address",
                "name": "refund",
                "type": "address"
              },
              {
                "internalType": "address",
                "name": "to",
                "type": "address"
              },
              {
                "internalType": "uint256",
                "name": "amount",
                "type": "uint256"
              },
              {
                "internalType": "bytes",
                "name": "variants",
                "type": "bytes"
              }
            ],
            "internalType": "struct Socket_Struct.Task_Params",
            "name": "params",
            "type": "tuple"
          }
        ],
        "indexed": false,
        "internalType": "struct Socket_Struct.Socket_Message",
        "name": "msg",
        "type": "tuple"
      }
    ],
    "name": "Socket",
    "type": "event"
  }
]
', 0, '{0.0.0-000014a34, 0.2.0-000bfc0, 0.3.0-00000008ffffffff00014a34c96971f6f5a1d20efcd465b1163812a955b414a3, 0.3.1-0000000000000000000000000000000000000000000000000000000000000000}', '0.3.4', '999500001', '<', '2024-08-02', '2024-08-02'),
(2, 'Bifrost_BRP_BTC_min_check', 49088, '0xc292D9d5c31D5246cfAC67ba91202bbCF0AA8108', '[
  {
    "anonymous": false,
    "inputs": [
      {
        "components": [
          {
            "components": [
              {
                "internalType": "ChainIndex",
                "name": "chain",
                "type": "bytes4"
              },
              {
                "internalType": "uint64",
                "name": "round_id",
                "type": "uint64"
              },
              {
                "internalType": "uint128",
                "name": "sequence",
                "type": "uint128"
              }
            ],
            "internalType": "struct Socket_Struct.RequestID",
            "name": "req_id",
            "type": "tuple"
          },
          {
            "internalType": "enum Socket_Struct.Task_Status",
            "name": "status",
            "type": "uint8"
          },
          {
            "components": [
              {
                "internalType": "ChainIndex",
                "name": "chain",
                "type": "bytes4"
              },
              {
                "internalType": "RBCmethod",
                "name": "method",
                "type": "bytes16"
              }
            ],
            "internalType": "struct Socket_Struct.Instruction",
            "name": "ins_code",
            "type": "tuple"
          },
          {
            "components": [
              {
                "internalType": "Asset_Index",
                "name": "tokenIDX0",
                "type": "bytes32"
              },
              {
                "internalType": "Asset_Index",
                "name": "tokenIDX1",
                "type": "bytes32"
              },
              {
                "internalType": "address",
                "name": "refund",
                "type": "address"
              },
              {
                "internalType": "address",
                "name": "to",
                "type": "address"
              },
              {
                "internalType": "uint256",
                "name": "amount",
                "type": "uint256"
              },
              {
                "internalType": "bytes",
                "name": "variants",
                "type": "bytes"
              }
            ],
            "internalType": "struct Socket_Struct.Task_Params",
            "name": "params",
            "type": "tuple"
          }
        ],
        "indexed": false,
        "internalType": "struct Socket_Struct.Socket_Message",
        "name": "msg",
        "type": "tuple"
      }
    ],
    "name": "Socket",
    "type": "event"
  }
]
', 0, '{0.0.0-00002711, 0.2.0-0000bfc0, 0.3.0-000000030000000100002711ffffffffffffffffffffffffffffffffffffffff}', '0.3.4', '999500001', '<', '2024-08-02', '2024-08-02');

-- Insert sample data into contract_event_block_log
INSERT INTO contract_event_block_log (id, block_number) VALUES (1, 19115020), (2, 19115020);