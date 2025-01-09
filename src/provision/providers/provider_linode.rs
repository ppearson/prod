/*
 Prod
 Copyright 2021-2025 Peter Pearson.
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
use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

use crate::provision::provision_provider::ProvisionProvider;
use crate::provision::provision_common::{ActionResultValues, ProvisionActionResult, ProvisionActionType, ProvisionResponseWaitType};
use crate::provision::provision_manager::ListType;
use crate::provision::provision_params::ProvisionParams;

use crate::column_list_printer::{ColumnListPrinter, Alignment};

#[derive(Serialize, Deserialize)]
struct TypeResultPrice {
    hourly: f32,
    monthly: f32,
}

#[derive(Serialize, Deserialize)]
struct TypeResultItem {
    id: String,
    label: String,
    class: String,
    memory: u32,
    disk: u32,
    transfer: u32,
    vcpus: u32,
    price: TypeResultPrice,
}

#[derive(Serialize, Deserialize)]
struct TypeListResults {
    data: Vec<TypeResultItem>
}

#[derive(Serialize, Deserialize)]
struct RegionResultItem {
    id: String,
    country: String,
}

#[derive(Serialize, Deserialize)]
struct RegionListResults {
    data: Vec<RegionResultItem>
}

#[derive(Serialize, Deserialize)]
struct ImageResultItem {
    id: String,
    label: String,
    deprecated: bool,
    size: u32,
    vendor: String,
}

#[derive(Serialize, Deserialize)]
struct ImageListResults {
    data: Vec<ImageResultItem>
}

#[derive(Clone, Debug)]
#[derive(Serialize, Deserialize)]
struct InstanceDetails {
    id:         u64,
    image:      String,

    ipv4:       Vec<String>,
    ipv6:       String,
    
    label:      String,

    status:     String,
}

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
        "linode".to_string()
    }

    fn supports_interactive(&self) -> bool {
        true
    }

    fn prompt_interactive(&self) -> Vec<(String, String)> {
        let mut items = Vec::new();
        items.push(("API_KEY".to_string(), "API Key to use Linode".to_string()));
        items
    }

    fn configure_interactive(&mut self) -> bool {
        false
    }

    fn configure(&mut self) -> bool {
        let linode_api_key_env = std::env::var("PROD_LINODE_API_KEY");
        match linode_api_key_env {
            Err(_e) => {
                // silently fail...
//                eprintln!("Error: $PROD_LINODE_API_KEY not set correctly.");
                return false;
            }
            Ok(v) => {
                self.linode_api_key = v.trim().to_string();
                return true;
            }
        }
    }

    fn is_configured(&self) -> bool {
        !self.linode_api_key.is_empty()
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
        let resp = ureq::get(url)
//            .set("Authorization", &format!("Bearer {}", self.linode_api_key))
            .call();
        
        if resp.is_err() {
            eprintln!("Error querying api.linode.com for list request: {:?}", resp.err());
            return false;
        }

        let resp_string = resp.unwrap().into_string().unwrap();

        if list_type == ListType::Regions {
            let results: RegionListResults = serde_json::from_str(&resp_string).unwrap();

            println!("{} regions:", results.data.len());

            let mut clp = ColumnListPrinter::new(2)
                .add_titles(["ID", "Country"]);

            for region in &results.data {
                clp.add_row_strings(&[&region.id, &region.country]);
            }

            print!("{}", clp);
        }
        else if list_type == ListType::Plans {
            let results: TypeListResults = serde_json::from_str(&resp_string).unwrap();

            println!("{} plans:", results.data.len());

            let mut clp = ColumnListPrinter::new(7)
                .set_alignment_multiple(&[2usize, 3, 4, 5, 6], Alignment::Right)
                .add_titles(["ID", "Label", "vcpus", "Memory", "Disk", "Transfer", "Price"]);

            for plan in &results.data {
                clp.add_row_strings(&[&plan.id, &plan.label, &format!("{}", plan.vcpus), &format!("{} MB", plan.memory), &format!("{} MB", plan.disk),
                                    &format!("{} MB", plan.transfer), &format!("${}", plan.price.monthly)]);
            }

            print!("{}", clp);
        }
        else if list_type == ListType::OSs {
            let results: ImageListResults = serde_json::from_str(&resp_string).unwrap();

            println!("{} OS images:", results.data.len());

            let mut clp = ColumnListPrinter::new(3)
                .add_titles(["ID", "Label", "Vendor"]);

            for image in &results.data {
                clp.add_row_strings(&[&image.id, &image.label, &image.vendor]);
            }

            print!("{}", clp);
        }
        else {
            println!("{}", resp_string);
        }

        true
    }

    fn get_required_params_for_action(&self, action: ProvisionActionType) -> BTreeSet<&str> {
        let mut params = BTreeSet::new();
        if action == ProvisionActionType::CreateInstance {
            params.insert("region");
            params.insert("type");
            params.insert("image");
            params.insert("label");
            params.insert("root_pass");
        }
        params
    }

    fn create_instance(&self, params: &ProvisionParams, _dry_run: bool) -> ProvisionActionResult {
        let region_str = params.get_string_value("region", "");
        let type_str = params.get_string_value("type", "");
        let label_str = params.get_string_value("label", "");
        let image_str = params.get_string_value("image", "");
        let root_pass_str = params.get_string_value("root_pass", "");
        let backups_enabled = params.get_string_value_as_bool("backups_enabled", false);

        // Note: get_string_array() will return even single strings as an array by-design...
        let authorized_keys = params.get_string_array("authorized_keys");
        let authorized_users = params.get_string_array("authorized_users");

        let mut json_value = ureq::json!({
            "region": region_str,
            "type": type_str,
            "label": label_str,
            "image": image_str,
            "root_pass": root_pass_str,
            "backups_enabled": backups_enabled,
        });

        if let Some(authorized_keys_array) = authorized_keys {
            json_value.as_object_mut().unwrap().insert("authorized_keys".to_string(), serde_json::to_value(authorized_keys_array).unwrap());
        }

        if let Some(authorized_users_array) = authorized_users {
            json_value.as_object_mut().unwrap().insert("authorized_users".to_string(), serde_json::to_value(authorized_users_array).unwrap());
        }

        let resp = ureq::post("https://api.linode.com/v4/linode/instances")
            .set("Authorization", &format!("Bearer {}", self.linode_api_key))
            .send_json(json_value);

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

        let instance_details = serde_json::from_str(&resp_string);
        if instance_details.is_err() {
            eprintln!("Error parsing json response from linode.com: {}", resp_string);
            return ProvisionActionResult::Failed("".to_string());
        }

        let instance_details: InstanceDetails = instance_details.unwrap();

        let mut result_values = ActionResultValues::new();

        let mut found_ip = false;

        // extract the values we want, and check there's roughly valid...
        // Note: we have to assume the 'id' value is a valid value here, as it's not clear
        //       what the default serde will provide is (I assume it would error if it's not there?)
        result_values.values.insert("id".to_string(), instance_details.id.to_string());

        // Note: the root password is specified via the params, so we know it...
        result_values.values.insert("root_password".to_string(), root_pass_str);

        if !instance_details.ipv4.is_empty() {
            found_ip = true;
            result_values.values.insert("ip".to_string(), instance_details.ipv4[0].clone());
        }

        eprintln!("Linode instance node created, id: {} ...", instance_details.id);

        if found_ip {
            eprintln!("Have instance IP: {}", instance_details.ipv4[0].clone());
        }

        if params.wait_type == ProvisionResponseWaitType::ReturnImmediatelyAfterAPIRequest {
            return ProvisionActionResult::ActionCreatedInProgress(result_values);
        }

        if found_ip && params.wait_type == ProvisionResponseWaitType::WaitForResourceCreationOrModification {
            // this is sufficient, so return out...
            return ProvisionActionResult::ActionCreatedInProgress(result_values);
        }

        eprintln!("Waiting for instance to spool up...");

        if instance_details.status == "provisioning" {
            let instance_id = result_values.values.get("id").unwrap().clone();

            let max_tries = 10;
            let mut try_count = 0;

            while try_count < max_tries {
                // sleep a bit to give things a chance...
                std::thread::sleep(std::time::Duration::from_secs(15));

                let instance_details = self.get_instance_details(&instance_id);
                if instance_details.is_err() {
                    eprintln!("Warning: Linode cloud instance was created, but received an unexpected json response4 from linode.com for get instance request: {}", resp_string);
                    return instance_details.err().unwrap();
                }
                let instance_details = instance_details.unwrap();

//              println!("InstanceDetails (t:{}) \n{:?}\n", try_count, instance_details);

                if !found_ip && !instance_details.ipv4.is_empty() {
                    // we now hopefully have a valid IP
                    found_ip = true;
                    result_values.values.insert("ip".to_string(), instance_details.ipv4[0].clone());

                    eprintln!("Have instance IP: {}", instance_details.ipv4[0].clone());

                    // so we now have an IP, but the instance still isn't ready to be used, but maybe that's
                    // all we need...
                    if params.wait_type == ProvisionResponseWaitType::WaitForResourceCreationOrModification {
                        // this is sufficient, so return out...
                        return ProvisionActionResult::ActionCreatedInProgress(result_values);
                    }

                    eprintln!("Waiting for server to finish install/setup...");
                }

                if instance_details.status == "running" {
                    // we should be done now...
                    break;
                }

                try_count += 1;
            }
        }
        
        ProvisionActionResult::ActionCreatedDone(result_values)
    }

    fn delete_instance(&self, params: &ProvisionParams, _dry_run: bool) -> ProvisionActionResult {
        let instance_id = params.get_string_value("instance_id", "");
        let full_url = format!("https://api.linode.com/v4/linode/instances/{}", instance_id);

        let resp = ureq::delete(&full_url)
        .set("Authorization", &format!("Bearer {}", self.linode_api_key))
            .call();

        // TODO: there's an insane amount of boilerplate error handling and response
        //       decoding going on here... Try and condense it...
        
        // TODO: make some of this re-useable for multiple actions...
        if resp.is_err() {
            match resp.err() {
                Some(Error::Status(code, response)) => {
                    // server returned an error code we weren't expecting...
                    match code {
                        400 => {
                            eprintln!("Error: Bad request error: {}", response.into_string().unwrap());
                            return ProvisionActionResult::ErrorAuthenticationIssue("".to_string());
                        },
                        401 => {
                            eprintln!("Error: authentication error with Linode API: {}", response.into_string().unwrap());
                            return ProvisionActionResult::ErrorAuthenticationIssue("".to_string());
                        },
                        404 => {
                            eprintln!("Error: Not found response from Linode API: {}", response.into_string().unwrap());
                            return ProvisionActionResult::Failed("".to_string());
                        }
                        _ => {
                            
                        }
                    }
                    eprintln!("Error deleting instance0: code: {}, resp: {:?}", code, response);
                },
                Some(e) => {
                    eprintln!("Error deleting instance1: {:?}", e);
                }
                _ => {
                    // some sort of transport/io error...
                    eprintln!("Error deleting instance2: ");
                }
            }
            return ProvisionActionResult::Failed("".to_string());
        }
        
        // response should be empty...
        let _resp_string = resp.unwrap().into_string().unwrap();

        ProvisionActionResult::ActionCreatedInProgress(ActionResultValues::new())
    }
}

impl ProviderLinode {
    fn get_instance_details(&self, instance_id: &str) -> Result<InstanceDetails, ProvisionActionResult> {
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

        let instance_details = serde_json::from_str(&resp_string);
        if instance_details.is_err() {
            eprintln!("Error parsing json response from linode.com for get instance call: {}", resp_string);
            return Err(ProvisionActionResult::Failed("".to_string()));
        }
        let instance_details: InstanceDetails = instance_details.unwrap();

        Ok(instance_details)
    }
}
