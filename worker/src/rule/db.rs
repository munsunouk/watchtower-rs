use std::collections::HashMap;

use crate::utils::error::WorkerError;

use watch_tower_lib::{
    rule::{
        contract_call::ContractCallRule, contract_event::ContractEventRule, rpc_call::RpcCallRule,
    },
    utils::evaluation::EvaluationRule,
};

/// # Description
/// This struct represents a difference between two contract event rules.
/// # Fields
/// * `new` - The new rules.
/// * `deleted` - The deleted rules.
/// * `updated` - The updated rules.
pub struct ContractEventDiff {
    pub new: Vec<ContractEventRule>,
    pub deleted: Vec<ContractEventRule>,
    pub updated: Vec<ContractEventRule>,
}

impl ContractEventDiff {
    pub fn is_changed(&self) -> bool {
        !self.new.is_empty() || !self.deleted.is_empty() || !self.updated.is_empty()
    }
}

/// # Description
/// This struct represents a difference between two contract call rules.
/// # Fields
/// * `new` - The new rules.
/// * `deleted` - The deleted rules.
/// * `updated` - The updated rules.
#[derive(Debug)]
pub struct ContractCallDiff {
    pub new: Vec<ContractCallRule>,
    pub deleted: Vec<ContractCallRule>,
    pub updated: Vec<ContractCallRule>,
}

impl ContractCallDiff {
    pub fn is_changed(&self) -> bool {
        !self.new.is_empty() || !self.deleted.is_empty() || !self.updated.is_empty()
    }
}

/// # Description
/// This struct represents a difference between two RPC call rules.
/// # Fields
/// * `new` - The new rules.
/// * `deleted` - The deleted rules.
/// * `updated` - The updated rules.
pub struct RpcCallDiff {
    pub new: Vec<RpcCallRule>,
    pub deleted: Vec<RpcCallRule>,
    pub updated: Vec<RpcCallRule>,
}

impl RpcCallDiff {
    pub fn is_changed(&self) -> bool {
        !self.new.is_empty() || !self.deleted.is_empty() || !self.updated.is_empty()
    }
}

/// # Description
/// This struct represents a difference between two evaluation rules.
/// # Fields
/// * `new` - The new rules.
/// * `deleted` - The deleted rules.
/// * `updated` - The updated rules.
pub struct EvaluationDiff {
    pub new: Vec<EvaluationRule>,
    pub deleted: Vec<EvaluationRule>,
    pub updated: Vec<EvaluationRule>,
}

impl EvaluationDiff {
    pub fn is_changed(&self) -> bool {
        !self.new.is_empty() || !self.deleted.is_empty() || !self.updated.is_empty()
    }
}

/// # Description
/// This struct represents a database rule.
/// # Fields
/// * `rpc_call_rules` - The RPC call rules.
/// * `contract_call_rules` - The contract call rules.
/// * `contract_event_rules` - The contract event rules.
/// * `evaluations` - The evaluation rules.
pub struct DBRule {
    pub rpc_call_rules: Vec<RpcCallRule>,
    pub contract_call_rules: Vec<ContractCallRule>,
    pub contract_event_rules: Vec<ContractEventRule>,
    pub evaluations: Vec<EvaluationRule>,
}

impl DBRule {
    pub fn new() -> Self {
        Self {
            rpc_call_rules: Vec::new(),
            contract_call_rules: Vec::new(),
            contract_event_rules: Vec::new(),
            evaluations: Vec::new(),
        }
    }

    // /// # Description
    // /// This function gets the difference between two RPC call rules.
    // /// # Arguments
    // /// * `new_rpc_call_rules` - The new RPC call rules.
    // /// # Returns
    // /// A `RpcCallDiff` struct.
    // pub fn get_rpc_call_diff(
    //     &self,
    //     new_rpc_call_rules: Vec<RpcCallRule>,
    // ) -> Result<RpcCallDiff, WorkerError> {
    //     let mut new_rules = Vec::new();
    //     let mut deleted_rules = Vec::new();
    //     let mut updated_rules = Vec::new();

    //     let mut existing_rules_map: HashMap<usize, &RpcCallRule> = self
    //         .rpc_call_rules
    //         .iter()
    //         .map(|rule| (rule.id, rule))
    //         .collect();

    //     // Identify new and updated rules
    //     for new_rule in new_rpc_call_rules {
    //         if let Some(existing_rule) = existing_rules_map.remove(&new_rule.id) {
    //             if new_rule != *existing_rule {
    //                 updated_rules.push(new_rule);
    //             }
    //         } else {
    //             new_rules.push(new_rule);
    //         }
    //     }

    //     // Remaining rules in existing_rules_map are deleted
    //     for (_, deleted_rule) in existing_rules_map {
    //         deleted_rules.push(deleted_rule.clone());
    //     }

    //     Ok(RpcCallDiff {
    //         new: new_rules,
    //         deleted: deleted_rules,
    //         updated: updated_rules,
    //     })
    // }

    // /// # Description
    // /// This function gets the difference between two contract call rules.
    // /// # Arguments
    // /// * `new_contract_call_rules` - The new contract call rules.
    // /// # Returns
    // /// A `ContractCallDiff` struct.
    // pub fn get_contract_call_diff(
    //     &self,
    //     new_contract_call_rules: Vec<ContractCallRule>,
    // ) -> Result<ContractCallDiff, WorkerError> {
    //     let mut new_rules = Vec::new();
    //     let mut deleted_rules = Vec::new();
    //     let mut updated_rules = Vec::new();

    //     let mut existing_rules_map: HashMap<usize, &ContractCallRule> = self
    //         .contract_call_rules
    //         .iter()
    //         .map(|rule| (rule.id, rule))
    //         .collect();

    //     // Identify new and updated rules
    //     for new_rule in new_contract_call_rules {
    //         if let Some(existing_rule) = existing_rules_map.remove(&new_rule.id) {
    //             if new_rule != *existing_rule {
    //                 updated_rules.push(new_rule);
    //             }
    //         } else {
    //             new_rules.push(new_rule);
    //         }
    //     }

    //     // Remaining rules in existing_rules_map are deleted
    //     for (_, deleted_rule) in existing_rules_map {
    //         deleted_rules.push(deleted_rule.clone());
    //     }

    //     Ok(ContractCallDiff {
    //         new: new_rules,
    //         deleted: deleted_rules,
    //         updated: updated_rules,
    //     })
    // }

    // /// # Description
    // /// This function gets the difference between two contract event rules.
    // /// # Arguments
    // /// * `new_contract_event_rules` - The new contract event rules.
    // /// # Returns
    // /// A `ContractEventDiff` struct.
    // pub fn get_contract_event_diff(
    //     &self,
    //     new_contract_event_rules: Vec<ContractEventRule>,
    // ) -> Result<ContractEventDiff, WorkerError> {
    //     let mut new_rules = Vec::new();
    //     let mut deleted_rules = Vec::new();
    //     let mut updated_rules = Vec::new();

    //     let mut existing_rules_map: HashMap<u32, HashMap<usize, &ContractEventRule>> = self
    //         .contract_event_rules
    //         .iter()
    //         .fold(HashMap::new(), |mut acc, rule| {
    //             acc.entry(rule.chain_id).or_default().insert(rule.id, rule);
    //             acc
    //         });

    //     // Identify new and updated rules
    //     for new_rule in new_contract_event_rules {
    //         if existing_rules_map.remove(&new_rule.chain_id).is_some() {
    //             updated_rules.push(new_rule);
    //         } else {
    //             new_rules.push(new_rule);
    //         }
    //     }

    //     // Remaining rules in existing_rules_map are deleted
    //     for (_, chain_rules) in existing_rules_map {
    //         for (_, deleted_rule) in chain_rules {
    //             deleted_rules.push(deleted_rule.clone());
    //         }
    //     }

    //     Ok(ContractEventDiff {
    //         new: new_rules,
    //         deleted: deleted_rules,
    //         updated: updated_rules,
    //     })
    // }

    /// # Description
    /// This function gets the difference between two evaluation rules.
    /// # Arguments
    /// * `new_evaluation_rules` - The new evaluation rules.
    /// # Returns
    /// A `EvaluationDiff` struct.
    pub fn get_evaluations_diff(
        &self,
        new_evaluation_rules: Vec<EvaluationRule>,
    ) -> Result<EvaluationDiff, WorkerError> {
        let mut new_evaluations = Vec::new();
        let mut deleted_evaluations = Vec::new();
        let mut updated_evaluations = Vec::new();

        let mut existing_evaluations_map: HashMap<usize, &EvaluationRule> = self
            .evaluations
            .iter()
            .map(|rule| (rule.id, rule))
            .collect();

        for new_rule in new_evaluation_rules {
            if existing_evaluations_map.remove(&new_rule.id).is_some() {
                updated_evaluations.push(new_rule);
            } else {
                new_evaluations.push(new_rule);
            }
        }

        // Remaining evaluations in existing_evaluations_map are deleted
        for (_, deleted_evaluation) in existing_evaluations_map {
            if deleted_evaluation.id == 0 {
                deleted_evaluations.push(deleted_evaluation.clone());
            } else {
                deleted_evaluations.insert(deleted_evaluation.id, deleted_evaluation.clone());
            }
        }

        Ok(EvaluationDiff {
            new: new_evaluations,
            deleted: deleted_evaluations,
            updated: updated_evaluations,
        })
    }
}
