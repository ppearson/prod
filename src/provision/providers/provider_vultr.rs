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
use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

use crate::provision::provision_provider::{ProvisionProvider};
use crate::provision::provision_common::{ActionResultValues, ProvisionActionResult, ProvisionActionType, ProvisionResponseWaitType};
use crate::provision::provision_manager::{ListType};
use crate::provision::provision_params::{ProvisionParams};

#[derive(Serialize, Deserialize)]
struct PlanResultItem {
    id: String,
    vcpu_count: u32,
    ram: u32,
    disk: u32,
    bandwidth: u32,
    monthly_cost: u32,

    #[serde(alias = "type")]
    plan_type: String,
}

#[derive(Serialize, Deserialize)]
struct PlanListResults {
    plans: Vec<PlanResultItem>
}

#[derive(Serialize, Deserialize)]
struct RegionResultItem {
    id: String,
    city: String,
    country: String,
    continent: String,
}

#[derive(Serialize, Deserialize)]
struct RegionListResults {
    regions: Vec<RegionResultItem>
}

#[derive(Serialize, Deserialize)]
struct OSResultItem {
    id: u32,
    name: String,
    arch: String,
    family: String,
}

#[derive(Serialize, Deserialize)]
struct OSListResults {
    os: Vec<OSResultItem>
}

#[derive(Serialize, Deserialize)]
struct InstanceDetails {
    instance: InstanceDetailsInner,
}

#[derive(Clone, Debug)]
#[derive(Serialize, Deserialize)]
struct InstanceDetailsInner {
    id:             String,
    os:             String,
    ram:            u32,
    disk:           u32,
    vcpu_count:     u32,

    main_ip:        String,
    v6_main_ip:     String,

    status:         String, // "active", "pending"
    power_status:   String, // "stopped", "running"
    server_status:  String, // "none", "locked", "installingbooting", // we never see this: "ok"
}

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
        let vultr_api_key_env = std::env::var("PROD_VULTR_API_KEY");
        match vultr_api_key_env {
            Err(_e) => {
                // silently fail...
//                eprintln!("Error: $PROD_VULTR_API_KEY not set correctly.");
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

        if list_type == ListType::Regions {
            let results: RegionListResults = serde_json::from_str(&resp_string).unwrap();

            // TODO: come up with some better way of doing this for column alignment...
            let max_city_length = results.regions.iter().map(|r| r.city.len()).max().unwrap();
            let max_country_length = results.regions.iter().clone().map(|r| r.country.len()).max().unwrap();

            println!("{} regions:", results.regions.len());

            for region in &results.regions {
                println!("{}  {:mcl$} {:mcntl$} {}", region.id, region.city, region.country, region.continent,
                                                    mcl = max_city_length, mcntl = max_country_length);
            }
        }
        else if list_type == ListType::Plans {
            let results: PlanListResults = serde_json::from_str(&resp_string).unwrap();

            // TODO: come up with some better way of doing this for column alignment...
            let max_id_length = results.plans.iter().map(|p| p.id.len()).max().unwrap();
            let max_cost_length = results.plans.iter().map(|p| format!("{}", p.monthly_cost).len()).max().unwrap();
            let max_disk_length = results.plans.iter().clone().map(|p| format!("{}", p.disk).len()).max().unwrap();
            let max_ram_length = results.plans.iter().clone().map(|p| format!("{}", p.ram).len()).max().unwrap();

            println!("{} plans:", results.plans.len());

            for plan in &results.plans {
                println!("{:midl$} : {:>mcl$} {:mdl$} GB {:mrl$} MB {} GB", plan.id, format!("${}", plan.monthly_cost),
                                                    plan.disk, plan.ram, plan.bandwidth,
                                                    midl = max_id_length, mcl = max_cost_length + 1,
                                                    mdl = max_disk_length, mrl = max_ram_length);
            }
        }
        else if list_type == ListType::OSs {
            let results: OSListResults = serde_json::from_str(&resp_string).unwrap();

            // TODO: come up with some better way of doing this for column alignment...
            let max_id_length = results.os.iter().map(|os| format!("{}", os.id).len()).max().unwrap();
            let max_name_length = results.os.iter().map(|os| os.name.len()).max().unwrap();
            let max_arch_length = results.os.iter().clone().map(|os| os.arch.len()).max().unwrap();
            let max_family_length = results.os.iter().clone().map(|os| os.family.len()).max().unwrap();

            println!("{} OS images:", results.os.len());

            for image in &results.os {
                println!("{}  {:mnl$} {:mal$} {:mfl$}", image.id, image.name, image.arch, image.family,
                                                    mnl = max_name_length, mal = max_arch_length, mfl = max_family_length);
            }
        }
        else {
            // TODO: format these nicely, and maybe filter them?...

            println!("{}", resp_string);
        }

        return true;
    }

    fn get_required_params_for_action(&self, action: ProvisionActionType) -> BTreeSet<&str> {
        let mut params = BTreeSet::new();
        if action == ProvisionActionType::CreateInstance {
            params.insert("region");
            params.insert("plan");
            params.insert("os_id");
        }
        else if action == ProvisionActionType::DeleteInstance {
            params.insert("instance_id");
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
                            eprintln!("Error: authentication error with Vultr API: {}", response.into_string().unwrap());
                            return ProvisionActionResult::ErrorAuthenticationIssue("".to_string());
                        },
                        404 => {
                            eprintln!("Error: Not found response from Vultr API: {}", response.into_string().unwrap());
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
            if let Some(val) = root_password_val {
                result_values.values.insert("root_password".to_string(), val.as_str().unwrap().to_string());
            }
        }
        else {
            eprintln!("Error: unexpected json response1 from Vultr.com: {}", resp_string);
            return ProvisionActionResult::Failed("".to_string());
        }

        let instance_id = result_values.values.get("id").unwrap().clone();

        eprintln!("Vultr instance created, id: {} ...", instance_id);

        if params.wait_type == ProvisionResponseWaitType::ReturnImmediatelyAfterAPIRequest {
            return ProvisionActionResult::ActionCreatedInProgress(result_values);
        }

        eprintln!("Waiting for instance to spool up...");

        // to get hold of the IP address, we need to do an additional API query to the
        // get instance API as it's still in the process of being spooled up..

        let max_tries = 10;
        let mut try_count = 0;

        let mut have_ip = false;
        let mut installing_booting_count = 0;

        while try_count < max_tries {
            // sleep a bit to give things a chance...
            std::thread::sleep(std::time::Duration::from_secs(15));

            let instance_details = self.get_instance_details(&instance_id);
            if instance_details.is_err() {
                eprintln!("Warning: Vultr cloud instance was created, but received an unexpected json response4 from vultr.com for get instance request: {}", resp_string);
                return instance_details.err().unwrap();
            }
            let instance_details = instance_details.unwrap().instance;

//            println!("InstanceDetails (t:{}) \n{:?}\n", try_count, instance_details);

            if !have_ip && instance_details.main_ip != "0.0.0.0" {
                // we now hopefully have a valid IP
                result_values.values.insert("ip".to_string(), instance_details.main_ip.clone());
                have_ip = true;

                eprintln!("Have instance IP: {}", instance_details.main_ip.clone());

                // so we now have an IP, but the instance still isn't ready to be used, but maybe that's
                // all we need...
                if params.wait_type == ProvisionResponseWaitType::WaitForResourceCreationOrModification {
                    // this is sufficient, so return out...
                    return ProvisionActionResult::ActionCreatedInProgress(result_values);
                }

                eprintln!("Waiting for server to finish install/setup...");
            }
            
            if params.wait_type == ProvisionResponseWaitType::WaitForResourceFinalised {
                // check the 'status' and 'server_status' fields.
                if instance_details.status == "active" && instance_details.power_status == "running" {
                    // Note: 'server_status' takes a while (~8 mins) to become 'ok', which isn't too helpful,
                    //       as the instances are generally ready after ~2 mins, at which point the
                    //       'server_status' value is still 'installingbooting', which is a bit annoying
                    //       for accurately working out when the instance is actually ready for use.

                    if instance_details.server_status == "installingbooting" {
                        
                        // this adds to the general wait time on purpose...
                        std::thread::sleep(std::time::Duration::from_secs(15));

                        // Note: also check we have a valid IP now, as it could be the situation (it has happened once or twice)
                        //       that provisioning the instance gets stuck, with no IP ever assigned, so we should try and guard
                        //       against that happening to some degree...
                        if have_ip && installing_booting_count > 2 {
                            // say we're done...
                            return ProvisionActionResult::ActionCreatedDone(result_values);
                        }

                        installing_booting_count += 1;
                    }
                }
            }

            try_count += 1;
        }

        if !have_ip {
            eprintln!("Warning: don't have an IP address yet, it's possible something went wrong...");
        }
        
        // work out what to do here... technically we have an instance, so...
        return ProvisionActionResult::ActionCreatedInProgress(result_values);
    }

    fn delete_instance(&self, params: &ProvisionParams, _dry_run: bool) -> ProvisionActionResult {
        let instance_id = params.get_value("instance_id", "");
        let full_url = format!("https://api.vultr.com/v2/instances/{}", instance_id);

        let resp = ureq::delete(&full_url)
            .set("Authorization", &format!("Bearer {}", self.vultr_api_key))
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
                            eprintln!("Error: authentication error with Vultr API: {}", response.into_string().unwrap());
                            return ProvisionActionResult::ErrorAuthenticationIssue("".to_string());
                        },
                        404 => {
                            eprintln!("Error: Not found response from Vultr API: {}", response.into_string().unwrap());
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

        return ProvisionActionResult::ActionCreatedInProgress(ActionResultValues::new());
    }
}

impl ProviderVultr {
    fn get_instance_details(&self, instance_id: &str) -> Result<InstanceDetails, ProvisionActionResult> {
        let url = format!("https://api.vultr.com/v2/instances/{}", &instance_id);
        let get_instance_response = ureq::get(&url)
            .set("Authorization", &format!("Bearer {}", self.vultr_api_key))
            .call();

        if let Err(error) = get_instance_response {
            let resp_string = error.to_string();
            eprintln!("Error parsing json response from Vultr.com for get instance call: {}", resp_string);
            return Err(ProvisionActionResult::Failed("".to_string()));
        }

        let resp_string = get_instance_response.unwrap().into_string().unwrap();

        let instance_details = serde_json::from_str(&resp_string);
        if instance_details.is_err() {
            eprintln!("Error parsing json response from Vultr.com for get instance call: {}", resp_string);
            return Err(ProvisionActionResult::Failed("".to_string()));
        }
        let instance_details: InstanceDetails = instance_details.unwrap();

        return Ok(instance_details);
    }
}
