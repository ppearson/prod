/*
 Prod
 Copyright 2021-2024 Peter Pearson.
 Licensed under the Apache License, Version 2.0 (the "License");
 You may not use this file except in compliance with the License.
 You may obtain a copy of the License at
 http://www.apache.org/licenses/LICENSE-2.0
 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License.
 ---------
*/

use std::collections::BTreeSet;

use super::provision_common::{ProvisionActionType, ProvisionActionResult};
use super::provision_provider::ProvisionProvider;

use super::providers::provider_binary_lane::ProviderBinaryLane;
use super::providers::provider_digital_ocean::ProviderDigitalOcean;
use super::providers::provider_linode::ProviderLinode;
use super::providers::provider_openstack::ProviderOpenStack;
use super::providers::provider_vultr::ProviderVultr;

use super::provision_params::ProvisionParams;

use crate::column_list_printer::ColumnListPrinter;

pub struct ProvisionManager {
    registered_providers: Vec<Box<dyn ProvisionProvider> >
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum ListType {
    Plans,
    Regions,
    OSs,
    Other
}

impl ProvisionManager {
    pub fn new() -> ProvisionManager {
        let mut manager = ProvisionManager { registered_providers: Vec::new() };

        // TODO: doing it this way is pretty silly.... we should lazily configure
        //       providers when needed, not configure them all ahead of time,
        //       as they each need different env variables / configuration...

        let mut new_provider = ProviderBinaryLane::new();
        new_provider.configure();
        manager.registered_providers.push(Box::new(new_provider));

        let mut new_provider = ProviderDigitalOcean::new();
        new_provider.configure();
        manager.registered_providers.push(Box::new(new_provider));

        let mut new_provider = ProviderLinode::new();
        new_provider.configure();
        manager.registered_providers.push(Box::new(new_provider));

        let mut new_provider = ProviderOpenStack::new();
        new_provider.configure();
        manager.registered_providers.push(Box::new(new_provider));

        let mut new_provider = ProviderVultr::new();
        new_provider.configure();
        manager.registered_providers.push(Box::new(new_provider));

        manager
    }

    fn find_provider(&self, provider: &str) -> Option<&dyn ProvisionProvider> {
        for prov in &self.registered_providers {
            if prov.name() == provider {
                return Some(prov.as_ref());
            }
        }

        None
    }

    pub fn list_available(&self, provider: &str, list_type: ListType) -> bool {
        let provider_item = self.find_provider(provider);

        if provider_item.is_none() {
            eprintln!("Error: Can't find provider: '{}'.", provider);
            return false;
        }

        let provider_item = provider_item.unwrap();

        // hopefully, providers don't need to be authenticated with their API key/token,
        // so we can just do anonymous GET requests...
        // DigitalOcean apparently needs to be :(

        provider_item.list_available(list_type);

        true
    }

    pub fn perform_action(&self, params: &ProvisionParams, dry_run: bool) -> ProvisionActionResult {
        if params.provider.is_empty() {
            eprintln!("Error: provider not specified.");
            return ProvisionActionResult::ErrorNotConfigured("".to_string());
        }

        let provider_item = self.find_provider(&params.provider);

        if provider_item.is_none() {
            eprintln!("Error: Can't find provider: '{}'.", params.provider);
            return ProvisionActionResult::ErrorNotConfigured("".to_string());
        }

        let provider_item = provider_item.unwrap();

        if !provider_item.is_configured() {
            eprintln!("Error: Provider for '{}' is not configured properly.", provider_item.name());
            return ProvisionActionResult::ErrorNotConfigured("".to_string());
        }
        
        let required_params = provider_item.get_required_params_for_action(params.action);
        if !self.check_required_params_are_provided(params, &required_params) {
            // Note: the function itself prints a helpful error message...
            return ProvisionActionResult::ErrorMissingParams("".to_string());
        }

        match params.action {
            ProvisionActionType::NotSet => {
                eprintln!("No action set.");
                return ProvisionActionResult::ErrorMissingParams("".to_string())
            },
            ProvisionActionType::CreateInstance => {
                let res = provider_item.create_instance(params, dry_run);
                match res.clone() {
                    ProvisionActionResult::ActionCreatedInProgress(res_values) |
                    ProvisionActionResult::ActionCreatedDone(res_values) => {
                        println!("Cloud instance created successfully:\n");
                        let mut clp = ColumnListPrinter::new(2);
                        for (key, val) in &res_values.values {
                            clp.add_row_strings(&[&format!("{}:", key), val.as_str()]);
                        }
                        println!("{}", clp);
                    },
                    _ => {           
                    }
                }
                return res;
            }
            ProvisionActionType::DeleteInstance => {
                let res = provider_item.delete_instance(params, dry_run);
                match res.clone() {
                    ProvisionActionResult::ActionCreatedInProgress(_res_values) |
                    ProvisionActionResult::ActionCreatedDone(_res_values) => {
                        println!("Cloud instance deleted successfully.\n");
                    },
                    _ => {       
                    }
                }
                return res;
            }
            _ => {
                
            }
        };

        ProvisionActionResult::Failed("".to_string())
    }

    // this will print user-friendly error itself, and just returns false to indicate calling code should early-out
    // if in error...
    fn check_required_params_are_provided(&self, params: &ProvisionParams, required_params: &BTreeSet<&str>) -> bool {
        if required_params.is_empty() {
            return true;
        }

        let mut missing_params = Vec::with_capacity(0);

        for required in required_params {
            if !params.has_param(required) {
                missing_params.push(required);
            }
        }

        if !missing_params.is_empty() {
            eprintln!("Error: required params are missing for the '{}' provider to perform the '{}' action:",
                         params.provider, params.action);

            for param in missing_params {
                eprintln!(" {}", param);
            }
        }

        true
    }
}
