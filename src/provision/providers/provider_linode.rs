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

pub struct ProviderLinode {
    linode_api_key: String,
}

impl ProviderLinode {
    pub fn new() -> ProviderLinode {
        ProviderLinode { linode_api_key: String::new() }
    }
}

impl ProvisionProvider for ProviderLinode {
    fn name(&self) -> String {
        return "linode".to_string();
    }

    fn supports_interactive(&self) -> bool {
        return true;
    }

    fn prompt_interactive(&self) -> Vec<(String, String)> {
        let mut items = Vec::new();
        items.push(("API_KEY".to_string(), "API Key to use Linode".to_string()));
        return items;
    }

    fn configure_interactive(&mut self) -> bool {
        return false;
    }

    fn configure(&mut self) -> bool {
        let linode_api_key_env = std::env::var("LINODE_API_KEY");
        match linode_api_key_env {
            Err(_e) => {
                // silently fail...
//                eprintln!("Error: $LINODE_API_KEY not set correctly.");
                return false;
            }
            Ok(v) => {
                self.linode_api_key = v.trim().to_string();
                return true;
            }
        }
    }

    fn is_configured(&self) -> bool {
        return !self.linode_api_key.is_empty();
    }

    // actual commands

    fn list_available(&self, list_type: ListType) -> bool {
        let url = match list_type {
            ListType::Plans => "https://api.linode.com/v4/linode/types",
            ListType::Regions => "https://api.linode.com/v4/regions",
            ListType::OSs => "https://api.linode.com/v4/images",
            _ => {
                return false;
            }
        };
        let resp = ureq::get(&url)
//            .set("Authorization", &format!("Bearer {}", self.linode_api_key))
            .call();
        
        if resp.is_err() {
            eprintln!("Error querying api.linode.com for list request: {:?}", resp.err());
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
            params.insert("type");
            params.insert("image");
//            params.insert("label"); // ? is this correct?
            params.insert("root_pass");
        }
        params
    }

    fn create_instance(&self, params: &ProvisionParams, _dry_run: bool) -> ProvisionActionResult {
        let region_str = params.get_value("region", "");
        let type_str = params.get_value("type", "");
        let label_str = params.get_value("label", "");
        let image_str = params.get_value("image", "");
        let root_pass_str = params.get_value("root_pass", "");

        let resp = ureq::post("https://api.linode.com/v4/linode/instances")
            .set("Authorization", &format!("Bearer {}", self.linode_api_key))
            .send_json(ureq::json!({
                "region": region_str,
                "type": type_str,
                "label": label_str,
                "image": image_str,
                "root_pass": root_pass_str,
            }));

        // TODO: there's an insane amount of boilerplate error handling and response
        //       decoding going on here... Try and condense it...
        
        // TODO: make some of this re-useable for multiple actions...
        if resp.is_err() {
            match resp.err() {
                Some(Error::Status(code, response)) => {
                    // server returned an error code we weren't expecting...
                    match code {
                        400 => {
                            eprintln!("Error: Bad request 400 error returned by Linode API: {}", response.into_string().unwrap());
                            eprintln!("Check that instance label does not exist already for an existing linode instance node.");
                            return ProvisionActionResult::Failed("".to_string());
                        }
                        401 => {
                            eprintln!("Error: authentication error with Linode API: {}", response.into_string().unwrap());
                            return ProvisionActionResult::ErrorAuthenticationIssue("".to_string());
                        },
                        404 => {
                            eprintln!("Error: 404 Not found response received from Linode API: {}", response.into_string().unwrap());
                            return ProvisionActionResult::Failed("".to_string());
                        }
                        _ => {
                            
                        }
                    }
                    eprintln!("Error creating instance0: code: {}, resp: {:?}", code, response.into_string().unwrap());
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
            eprintln!("Error parsing json response from linode.com: {}", resp_string);
            return ProvisionActionResult::Failed("".to_string());
        }

        let mut result_values = ActionResultValues::new();

        let parsed_value_map = parsed_response.ok().unwrap();
//        eprintln!("Created Linode instance okay:\n{:?}", parsed_value_map);

        let mut status_str = String::new();

        // check it's an array object and other stuff (i.e. check the json is expected)
        if parsed_value_map.is_object() {
            let value_as_object = parsed_value_map.as_object().unwrap();
           
            // extract the values we want
            let id_val = value_as_object.get("id");
            match id_val {
                Some(val) => {
                    result_values.values.insert("id".to_string(), val.as_u64().unwrap().to_string());
                },
                _ => {
                    eprintln!("Error: unexpected json response from linode.com - missing 'id' param: {}", resp_string);
                    return ProvisionActionResult::Failed("".to_string());
                }
            }
            let ip_v4_val = value_as_object.get("ipv4");
            let mut found_ip = false;
            match ip_v4_val {
                Some(val) => {
                    if val.is_array() {
                        let ip_array = val.as_array().unwrap();
                        if ip_array.len() > 0 {
                            let ip_address = ip_array[0].as_str().unwrap().to_string();
                            result_values.values.insert("ip".to_string(), ip_address);
                            found_ip = true;
                        }
                    }
                    
                },
                _ => {}
            }

            if !found_ip {
                eprintln!("Error: couldn't find ipv4 address in json response from Linode to create node:\n{}", resp_string);
                return ProvisionActionResult::Failed("".to_string());
            }

            // get status string - it's almost certainly 'provisioning', but we should probably check it just in case
            let status_val = value_as_object.get("status");
            match status_val {
                Some(val) => {
                    if val.is_string() {
                        status_str = val.as_str().unwrap().to_string();
                    }
                },
                _ => {}
            }
        }
        else {
            eprintln!("Error: unexpected json response1 from linode.com: {}", resp_string);
            return ProvisionActionResult::Failed("".to_string());
        }

        eprintln!("Linode instance node created...");

        if status_str == "provisioning" {
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
                let main_ip_val = instance_info_map.get("status");
                match main_ip_val {
                    Some(val) => {
                        status_str = val.as_str().unwrap().to_string();
                        if status_str.as_str() == "running" {
                            break;
                        }                        
                    },
                    _ => {
                        eprintln!("Warning: Linode cloud instance was created, but received an unexpected json response4 from linode.com for get instance request - status param not known: {}", resp_string);
                    }
                }

                try_count += 1;
            }
        }
        
        return ProvisionActionResult::ActionCreatedDone(result_values);
    }
}

impl ProviderLinode {
    fn get_value_map_from_get_instance_call(&self, instance_id: &str) -> Result<serde_json::Map<String, Value>, ProvisionActionResult> {
        let url = format!("https://api.linode.com/v4/linode/instances/{}", &instance_id);
        let get_instance_response = ureq::get(&url)
            .set("Authorization", &format!("Bearer {}", self.linode_api_key))
            .call();

        if let Err(error) = get_instance_response {
            let resp_string = error.to_string();
            eprintln!("Error parsing json response from linode.com for get instance call: {}", resp_string);
            return Err(ProvisionActionResult::Failed("".to_string()));
        }

        let resp_string = get_instance_response.unwrap().into_string().unwrap();
        let parsed_response = serde_json::from_str::<Value>(&resp_string);
        if parsed_response.is_err() {
            eprintln!("Error parsing json response from linode.com for get instance call: {}", resp_string);
            return Err(ProvisionActionResult::Failed("".to_string()));
        }

        let parsed_value_map = parsed_response.ok().unwrap();
        if parsed_value_map.is_object() {
            let value_as_object = parsed_value_map.as_object().unwrap();
           
            let response_map = value_as_object.clone();
            return Ok(response_map);
        }

        return Err(ProvisionActionResult::Failed("".to_string()))
    }
}
