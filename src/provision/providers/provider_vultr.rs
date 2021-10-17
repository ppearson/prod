/*
 Prod
 Copyright 2021 Peter Pearson.
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

use ureq;
use ureq::Error;
use serde_json::{Value};

use std::collections::BTreeSet;

use crate::provision::provision_provider::{ProvisionProvider};
use crate::provision::provision_common::{ProvisionActionType, ProvisionActionResult, ActionResultValues};
use crate::provision::provision_manager::{ListType};
use crate::provision::provision_params::{ProvisionParams};

pub struct ProviderVultr {
    vultr_api_key: String,
}

impl ProviderVultr {
    pub fn new() -> ProviderVultr {
        ProviderVultr { vultr_api_key: String::new() }
    }
}

impl ProvisionProvider for ProviderVultr {
    fn name(&self) -> String {
        return "vultr".to_string();
    }

    fn supports_interactive(&self) -> bool {
        return true;
    }

    fn prompt_interactive(&self) -> Vec<(String, String)> {
        let mut items = Vec::new();
        items.push(("API_KEY".to_string(), "API Key to use Vultr".to_string()));
        return items;
    }

    fn configure_interactive(&mut self) -> bool {
        return false;
    }

    fn configure(&mut self) -> bool {
        let vultr_api_key_env = std::env::var("VULTR_API_KEY");
        match vultr_api_key_env {
            Err(_e) => {
                // silently fail...
//                eprintln!("Error: $VULTR_API_KEY not set correctly.");
                return false;
            }
            Ok(v) => {
                self.vultr_api_key = v.trim().to_string();
                return true;
            }
        }
    }

    fn is_configured(&self) -> bool {
        return !self.vultr_api_key.is_empty();
    }

    // actual commands

    fn list_available(&self, list_type: ListType) -> bool {
        let url = match list_type {
            ListType::Plans => "plans",
            ListType::Regions => "regions",
            ListType::OSs => "os",
            _ => {
                return false;
            }
        };
        let resp = ureq::get(&format!("https://api.vultr.com/v2/{}", url))
//            .set("Authorization", &format!("Bearer {}", self.vultr_api_key))
            .call();
        
        if resp.is_err() {
            eprintln!("Error querying api.vultr.com for list request.");
            return false;
        }

        let resp_string = resp.unwrap().into_string().unwrap();

        // TODO: format these nicely, and maybe filter them?...

        println!("{}", resp_string);
        
        return true;
    }

    fn get_required_params_for_action(&self, action: ProvisionActionType) -> BTreeSet<&str> {
        let mut params = BTreeSet::new();
        if action == ProvisionActionType::CreateInstance {
            params.insert("region");
            params.insert("plan");
            params.insert("os_id");
        }
        params
    }

    fn create_instance(&self, params: &ProvisionParams, _dry_run: bool) -> ProvisionActionResult {
        let region_str = params.get_value("region", "");
        let plan_str = params.get_value("plan", "");
        let label_str = params.get_value("label", "");
        let os_id_str = params.get_value("os_id", "");
        let os_id = os_id_str.parse::<u32>().unwrap();
        let enable_ipv6 = params.get_value_as_bool("enable_ipv6", false);

        let resp = ureq::post("https://api.vultr.com/v2/instances")
            .set("Authorization", &format!("Bearer {}", self.vultr_api_key))
            .send_json(ureq::json!({
                "region": region_str,
                "plan": plan_str,
                "label": label_str,
                "os_id": os_id,
                "enable_ipv6": enable_ipv6,
            }));

        // TODO: there's an insane amount of boilerplate error handling and response
        //       decoding going on here... Try and condense it...
        
        // TODO: make some of this re-useable for multiple actions...
        if resp.is_err() {
            match resp.err() {
                Some(Error::Status(code, response)) => {
                    // server returned an error code we weren't expecting...
                    match code {
                        401 => {
                            eprintln!("Error: authentication error with Vultr API: {}", response.into_string().unwrap().clone());
                            return ProvisionActionResult::ErrorAuthenticationIssue("".to_string());
                        },
                        404 => {
                            eprintln!("Error: Not found response from Vultr API: {}", response.into_string().unwrap().clone());
                            return ProvisionActionResult::Failed("".to_string());
                        }
                        _ => {
                            
                        }
                    }
                    eprintln!("Error creating instance0: code: {}, resp: {:?}", code, response);
                },
                Some(e) => {
                    eprintln!("Error creating instance1: {:?}", e);
                }
                _ => {
                    // some sort of transport/io error...
                    eprintln!("Error creating instance2: ");
                }
            }
            return ProvisionActionResult::Failed("".to_string());
        }
        
        let resp_string = resp.unwrap().into_string().unwrap();
        let parsed_response = serde_json::from_str::<Value>(&resp_string);
        if parsed_response.is_err() {
            eprintln!("Error parsing json response from Vultr.com: {}", resp_string);
            return ProvisionActionResult::Failed("".to_string());
        }

        let mut result_values = ActionResultValues::new();

        let parsed_value_map = parsed_response.ok().unwrap();
//        eprintln!("Created Vultr instance okay:\n{:?}", parsed_value_map);

        // check it's an array object and other stuff (i.e. check the json is expected)
        if parsed_value_map.is_object() {
            let value_as_object = parsed_value_map.as_object().unwrap();
            // we only expect 1 actual instance value...
            let instance_map = value_as_object.get("instance");
            if instance_map.is_none() {
                eprintln!("Error: unexpected json response2 from Vultr.com: {}", resp_string);
                return ProvisionActionResult::Failed("".to_string());
            }
            let instance_map = instance_map.unwrap();
    
            // otherwise, hopefully we have what we need...
//            eprintln!("\nSingular response: {:?}", instance_map);

            // extract the values we want
            let id_val = instance_map.get("id");
            match id_val {
                Some(val) => {
                    result_values.values.insert("id".to_string(), val.as_str().unwrap().to_string());
                },
                _ => {
                    eprintln!("Error: unexpected json response3 from Vultr.com - missing 'id' param: {}", resp_string);
                    return ProvisionActionResult::Failed("".to_string());
                }
            }
            let root_password_val = instance_map.get("default_password");
            match root_password_val {
                Some(val) => {
                    result_values.values.insert("root_password".to_string(), val.as_str().unwrap().to_string());
                },
                _ => {}
            }
        }
        else {
            eprintln!("Error: unexpected json response1 from Vultr.com: {}", resp_string);
            return ProvisionActionResult::Failed("".to_string());
        }

        eprintln!("Vultr instance created...");
        eprintln!("Waiting for instance to spool up with IP address...");

        // to get hold of the IP address, we need to do an additional API query to the
        // get instance API as it's still in the process of being spooled up..

        let instance_id = result_values.values.get("id").unwrap();

        let max_tries = 5;
        let mut try_count = 0;

        while try_count < max_tries {
            // sleep a bit to give things a chance...
            std::thread::sleep(std::time::Duration::from_secs(15));

            let instance_info = self.get_value_map_from_get_instance_call(&instance_id);
            if instance_info.is_err() {
                return instance_info.err().unwrap();
            }
            let instance_info_map = instance_info.unwrap();

            // extract the values we want
            let main_ip_val = instance_info_map.get("main_ip");
            match main_ip_val {
                Some(val) => {
                    match val.as_str() {
                        Some("0.0.0.0") => {
                            // hasn't spun up yet...
                        },
                        Some(ip_val) => {
                            // hopefully the actual IP
                            result_values.values.insert("ip".to_string(), ip_val.to_string());
                            // break out as we have it...
                            break;
                        },
                        None => {
                            // something went wrong...
                            eprintln!("Error: unexpected json response1 from vultr.com: {}", resp_string);
                            return ProvisionActionResult::Failed("".to_string());
                        }
                    }
                    
                },
                _ => {
                    eprintln!("Warning: Vultr cloud instance was created, but received an unexpected json response4 from vultr.com for get instance request - missing 'main_ip' param: {}", resp_string);
                }
            }

            try_count += 1;
        }
        
        return ProvisionActionResult::ActionCreatedInProgress(result_values);
    }
}

impl ProviderVultr {
    fn get_value_map_from_get_instance_call(&self, instance_id: &str) -> Result<serde_json::Value, ProvisionActionResult> {
        let url = format!("https://api.vultr.com/v2/instances/{}", &instance_id);
        let get_instance_response = ureq::get(&url)
            .set("Authorization", &format!("Bearer {}", self.vultr_api_key))
            .call();

        if get_instance_response.is_err() {
            let resp_string = get_instance_response.unwrap().into_string().unwrap();
            eprintln!("Error parsing json response from Vultr.com for get instance call: {}", resp_string);
            return Err(ProvisionActionResult::Failed("".to_string()));
        }

        let resp_string = get_instance_response.unwrap().into_string().unwrap();
        let parsed_response = serde_json::from_str::<Value>(&resp_string);
        if parsed_response.is_err() {
            eprintln!("Error parsing json response from Vultr.com for get instance call: {}", resp_string);
            return Err(ProvisionActionResult::Failed("".to_string()));
        }

        let parsed_value_map = parsed_response.ok().unwrap();
        if parsed_value_map.is_object() {
            let value_as_object = parsed_value_map.as_object().unwrap();
            // we only expect 1 actual instance value...
            let instance_map = value_as_object.get("instance");
            if instance_map.is_none() {
                eprintln!("Error: unexpected json response2 from Vultr.com for get instance call: {}", resp_string);
                return Err(ProvisionActionResult::Failed("".to_string()));
            }
            let instance_map = instance_map.unwrap().clone();

            return Ok(instance_map);
        }

        return Err(ProvisionActionResult::Failed("".to_string()))
    }
}
